use chrono::DateTime;
use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned, Params};
use std::sync::Arc;
use tracing::info;
use tx3_cardano::ChainPoint;
use tx3_resolver::trp;

use crate::trp::Context;

pub async fn execute(
    params: Params<'_>,
    context: Arc<Context>,
) -> Result<serde_json::Value, ErrorObjectOwned> {
    info!(method = "trp.resolve", "Received TRP request.");

    let request: trp::ResolveParams = params.parse()?;
    let (tx, args) = trp::parse_resolve_request(request).map_err(|x| {
        ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "Failed to parse resolve request",
            Some(x.to_string()),
        )
    })?;

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

    let progress = hydra.get_progress().await;

    let timestamp = if progress.timestamp.is_empty() {
        0u64
    } else {
        let dt = DateTime::parse_from_rfc3339(&progress.timestamp).map_err(|e| {
            ErrorObject::owned(
                ErrorCode::InternalError.code(),
                "Failed to parse timestamp",
                Some(e.to_string()),
            )
        })?;
        dt.timestamp_millis() as u64
    };

    let mut compiler = tx3_cardano::Compiler::new(
        pparams,
        tx3_cardano::Config {
            extra_fees: Some(0),
        },
        ChainPoint {
            slot: progress.seq,
            hash: vec![0; 32],
            timestamp: timestamp as u128,
        },
    );

    let resolved = tx3_resolver::resolve_tx(
        tx,
        &args,
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
