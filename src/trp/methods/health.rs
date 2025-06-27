use std::sync::Arc;

use jsonrpsee::types::ErrorObjectOwned;

use crate::trp::Context;

pub async fn execute(context: Arc<Context>) -> Result<bool, ErrorObjectOwned> {
    Ok(context.hydra_adapter.check_health().await)
}
