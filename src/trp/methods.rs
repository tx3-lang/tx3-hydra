use std::sync::Arc;

use jsonrpsee::types::{ErrorObjectOwned, Params};

use super::Context;

pub async fn trp_resolve(
    _params: Params<'_>,
    _context: Arc<Context>,
) -> Result<serde_json::Value, ErrorObjectOwned> {
    // TODO: implement resolve
    Ok(serde_json::Value::default())
}

pub fn health(_context: &Context) -> bool {
    // TODO: implement health check
    true
}
