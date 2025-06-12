use std::sync::Arc;

use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned, Params};
use resolve::decode_params;
use tracing::info;

use super::Context;

mod resolve;

pub async fn trp_resolve(
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

    let hydra_adapter = context.hydra_adapter.clone();
    let hydra_ledger = hydra_adapter.ledger.read().await.clone();

    let resolved =
        tx3_cardano::resolve_tx(tx, hydra_ledger, context.config.max_optimize_rounds.into())
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

pub fn health(_context: &Context) -> bool {
    // TODO: implement hydra/trp health check
    true
}
