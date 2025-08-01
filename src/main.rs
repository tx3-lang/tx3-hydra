use std::{env, sync::Arc};

use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{Level, debug};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod hydra;
mod trp;

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

    let cancellation_token = cancellation_token();

    let (hydra_channel, _) = tokio::sync::broadcast::channel::<hydra::model::Event>(1);

    let hydra_channel = Arc::new(hydra_channel);

    let hydra_adapter = Arc::new(
        hydra::HydraAdapter::try_new(config.hydra.clone(), Arc::clone(&hydra_channel)).await?,
    );

    let hydra_subscribe = hydra_adapter.subscribe(cancellation_token.clone());
    let trp_server = trp::run(
        config.trp.clone(),
        Arc::clone(&hydra_adapter),
        Arc::clone(&hydra_channel),
        cancellation_token.clone(),
    );

    tokio::try_join!(hydra_subscribe, trp_server)?;

    Ok(())
}

#[derive(Deserialize, Clone)]
pub struct Config {
    trp: trp::Config,
    hydra: hydra::Config,
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

fn cancellation_token() -> CancellationToken {
    let cancel = CancellationToken::new();

    let cancel_cloned = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl+C");
        debug!("shutdown signal received");
        cancel_cloned.cancel();
    });

    cancel
}
