use std::sync::Arc;

use jsonrpsee::{RpcModule, server::Server};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::hydra::{self, HydraAdapter};

mod mapping;
mod methods;
mod utxos;

pub async fn run(
    config: Config,
    hydra_adapter: Arc<HydraAdapter>,
    hydra_channel: Arc<broadcast::Sender<hydra::model::Event>>,
    cancellation_token: CancellationToken,
) -> anyhow::Result<()> {
    let cors_layer = if config.permissive_cors {
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
    };

    let middleware = ServiceBuilder::new().layer(cors_layer);
    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(&config.listen_address)
        .await?;

    let mut module = RpcModule::new(Context {
        hydra_adapter,
        config: config.clone(),
    });

    module.register_async_method("trp.resolve", |params, context, _| async {
        methods::resolve::execute(params, context).await
    })?;

    module.register_async_method("trp.submit", move |params, context, _| {
        let hydra_channel = Arc::clone(&hydra_channel);
        async move { methods::submit::execute(params, context, hydra_channel).await }
    })?;

    module.register_async_method("health", |_, context, _| async {
        methods::health::execute(context).await
    })?;

    info!(
        address = config.listen_address.to_string(),
        "TRP server running"
    );

    let handle = server.start(module);

    let server = async {
        handle.clone().stopped().await;
        Ok::<(), anyhow::Error>(())
    };

    let cancellation = async {
        cancellation_token.cancelled().await;
        info!("gracefully shuting down trp");
        let _ = handle.stop();
        Ok::<(), anyhow::Error>(())
    };

    tokio::try_join!(server, cancellation)?;

    Ok(())
}

struct Context {
    hydra_adapter: Arc<HydraAdapter>,
    config: Config,
}

fn default_max_optimize_rounds() -> usize {
    10
}

#[derive(Deserialize, Clone)]
pub struct Config {
    listen_address: String,
    #[serde(default)]
    permissive_cors: bool,
    #[serde(default = "default_max_optimize_rounds")]
    max_optimize_rounds: usize,
}
