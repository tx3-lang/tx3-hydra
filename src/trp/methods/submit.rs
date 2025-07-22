use std::sync::Arc;

use base64::{Engine, prelude::BASE64_STANDARD};
use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned, Params};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use tx3_cardano::pallas::ledger::traverse::MultiEraTx;

use crate::{
    hydra::model::{HydraMessage, NewTx},
    trp::Context,
};

use super::Encoding;

#[derive(Deserialize)]
pub struct TrpSubmitTxRequest {
    pub encoding: Encoding,
    pub payload: String,
}

#[derive(Deserialize)]
pub struct TrpSubmitRequest {
    pub tx: TrpSubmitTxRequest,
}

#[derive(Serialize)]
pub struct TrpSubmitResponse {
    pub hash: String,
}

pub async fn execute(
    params: Params<'_>,
    context: Arc<Context>,
) -> Result<serde_json::Value, ErrorObjectOwned> {
    tracing::info!(method = "trp.submit", "Received TRP request.");

    let request = params.parse::<TrpSubmitRequest>().map_err(|error| {
        error!(?error);
        ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "invalid params",
            Some(error.to_string()),
        )
    })?;

    let raw = match request.tx.encoding {
        Encoding::Hex => hex::decode(request.tx.payload).map_err(|error| {
            error!(?error);
            ErrorObject::owned(
                ErrorCode::ParseError.code(),
                "invalid tx hex encoding",
                Some(error.to_string()),
            )
        })?,
        Encoding::Base64 => BASE64_STANDARD
            .decode(request.tx.payload)
            .map_err(|error| {
                error!(?error);
                ErrorObject::owned(
                    ErrorCode::ParseError.code(),
                    "invalid tx base64 encoding",
                    Some(error.to_string()),
                )
            })?,
    };

    let metx = MultiEraTx::decode(&raw).map_err(|error| {
        error!(?error);
        ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "failed to decode tx",
            Some(error.to_string()),
        )
    })?;

    if !metx.is_valid() {
        return Err(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            "invalid tx",
            None as Option<String>,
        ));
    }

    let hash = metx.hash();

    let message = HydraMessage::NewTx(NewTx::new(raw));
    context
        .hydra_adapter
        .submit(message)
        .await
        .map_err(|error| {
            error!(?error);
            ErrorObject::owned(
                ErrorCode::InternalError.code(),
                "failed sending tx to hydra",
                Some(error.to_string()),
            )
        })?;

    let hash = hex::encode(hash);
    info!(?hash, "submitting tx");
    let response = serde_json::to_value(TrpSubmitResponse { hash }).map_err(|error| {
        error!(?error);
        ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "transaction accepted, but error to encode response",
            Some(error.to_string()),
        )
    })?;

    Ok(response)
}
