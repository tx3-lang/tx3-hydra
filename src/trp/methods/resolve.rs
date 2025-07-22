use base64::{Engine, engine::general_purpose::STANDARD};
use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use tx3_lang::ProtoTx;

use crate::trp::Context;

use super::Encoding;

#[derive(Deserialize, Debug)]
struct IrEnvelope {
    #[allow(dead_code)]
    pub version: String,
    pub bytecode: String,
    pub encoding: Encoding,
}

#[derive(Deserialize, Debug)]
struct TrpResolveParams {
    pub tir: IrEnvelope,
    pub args: serde_json::Value,
}

fn handle_param_args(tx: &mut ProtoTx, args: &serde_json::Value) -> Result<(), ErrorObjectOwned> {
    let Some(arguments) = args.as_object() else {
        return Err(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "Failed to parse arguments as object.",
            None as Option<String>,
        ));
    };

    let params = tx.find_params();

    for (key, ty) in params {
        let Some(arg) = arguments.get(&key) else {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("Missing argument for parameter {key} of type {ty:?}"),
                Some(serde_json::json!({ "key": key, "type": ty })),
            ));
        };

        let arg = tx3_sdk::trp::args::from_json(arg.clone(), &ty).map_err(|e| {
            ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("Failed to parse argument {key} of type {ty:?}"),
                Some(serde_json::json!({ "error": e.to_string(), "value": arg })),
            )
        })?;

        tx.set_arg(&key, arg);
    }

    Ok(())
}

fn decode_params(params: Params<'_>) -> Result<ProtoTx, ErrorObjectOwned> {
    let params: TrpResolveParams = params.parse()?;

    if params.tir.version != tx3_lang::ir::IR_VERSION {
        return Err(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            format!(
                "Unsupported IR version, expected {}. Make sure you have the latest version of the tx3 toolchain",
                tx3_lang::ir::IR_VERSION
            ),
            Some(params.tir.version),
        ));
    }

    let tx = match params.tir.encoding {
        Encoding::Base64 => STANDARD.decode(params.tir.bytecode).map_err(|x| {
            ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "Failed to decode IR using Base64 encoding",
                Some(x.to_string()),
            )
        })?,
        Encoding::Hex => hex::decode(params.tir.bytecode).map_err(|x| {
            ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "Failed to decode IR using hex encoding",
                Some(x.to_string()),
            )
        })?,
    };

    let mut tx = tx3_lang::ProtoTx::from_ir_bytes(&tx).map_err(|x| {
        ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "Failed to decode IR bytes",
            Some(x.to_string()),
        )
    })?;

    handle_param_args(&mut tx, &params.args)?;

    Ok(tx)
}

pub async fn execute(
    params: Params<'_>,
    context: Arc<Context>,
) -> Result<serde_json::Value, ErrorObjectOwned> {
    info!(method = "trp.resolve", "Received TRP request.");
    let tx = match decode_params(params) {
        Ok(tx) => tx,
        Err(err) => {
            tracing::warn!(err = ?err, "Failed to decode params.");
            return Err(err);
        }
    };

    let hydra = context.hydra_adapter.clone();
    let utxos = hydra.read_utxos().await;

    // TODO: very inefficient to query it time we resolve a tx
    let pparams = hydra.get_pparams().await.map_err(|e| {
        ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "Failed to get pparams",
            Some(e.to_string()),
        )
    })?;

    let mut compiler = tx3_cardano::Compiler::new(
        pparams,
        tx3_cardano::Config {
            extra_fees: Some(0),
        },
    );

    let resolved = tx3_resolver::resolve_tx(
        tx,
        &mut compiler,
        &utxos,
        context.config.max_optimize_rounds,
    )
    .await
    .map_err(|err| {
        ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "Failed to resolve",
            Some(err.to_string()),
        )
    });

    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(err) => {
            tracing::warn!(err = ?err, "Failed to resolve tx.");
            return Err(err);
        }
    };

    Ok(serde_json::json!({ "tx": hex::encode(resolved.payload) }))
}
