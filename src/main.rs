use std::env;

use jsonrpsee::{RpcModule, server::Server};
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod methods;

#[tokio::main()]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .with_env_var("RUST_LOG")
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();

    let config = Config::new()?;

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
        config: config.clone(),
    });

    module.register_async_method("trp.resolve", |params, context, _| async {
        methods::trp_resolve(params, context).await
    })?;

    module.register_method("health", |_, context, _| methods::health(context))?;

    info!(
        address = config.listen_address.to_string(),
        "TRP server running"
    );

    server.start(module).stopped().await;

    Ok(())
}

struct Context {
    #[allow(dead_code)]
    config: Config,
}

#[derive(Deserialize, Clone)]
struct Config {
    listen_address: String,
    #[serde(default)]
    permissive_cors: bool,
}
impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let config: Config = config::Config::builder()
            .add_source(
                config::File::with_name(
                    &env::var("TRP_HYDRA_CONFIG").unwrap_or("config.toml".into()),
                )
                .required(false),
            )
            .add_source(config::File::with_name("/etc/tx3hydra/config.toml").required(false))
            .add_source(config::Environment::with_prefix("TRP_HYDRA").separator("_"))
            .build()?
            .try_deserialize()?;

        Ok(config)
    }
}
