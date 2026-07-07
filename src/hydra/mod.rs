use anyhow::Context;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock, RwLockReadGuard, broadcast},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use tx3_cardano::PParams;

pub mod model;
pub mod state;

use model::{Event, EventMeta, HydraMessage, HydraPParams};
use state::HeadState;
pub use state::Progress;

pub struct UtxoSnapshot<'a>(pub RwLockReadGuard<'a, HeadState>);

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct HydraAdapter {
    config: Config,
    state: RwLock<HeadState>,
    stream: Mutex<SplitStream<WsStream>>,
    sink: Mutex<SplitSink<WsStream, Message>>,
    hydra_channel: Arc<broadcast::Sender<Event>>,
}

impl HydraAdapter {
    pub async fn try_new(
        config: Config,
        hydra_channel: Arc<broadcast::Sender<Event>>,
    ) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(&config.ws_url).await?;
        info!("Hydra ws handshake has been successfully completed");

        let (write, read) = ws_stream.split();

        let state = RwLock::new(HeadState::default());
        let stream = Mutex::new(read);
        let sink = Mutex::new(write);

        Ok(Self {
            config,
            state,
            stream,
            sink,
            hydra_channel,
        })
    }

    pub async fn subscribe(&self, cancellation_token: CancellationToken) -> anyhow::Result<()> {
        info!("Listening Hydra events");

        let mut stream = self.stream.lock().await;
        let message_processing = async {
            while let Some(result) = stream.next().await {
                let message = result?;

                if message.is_close() {
                    info!("Received WebSocket close message");
                    break;
                }

                let text = message.to_text().unwrap();

                match serde_json::from_str::<Event>(text) {
                    Ok(event) => {
                        let meta = serde_json::from_str::<EventMeta>(text).unwrap_or_default();
                        self.state.write().await.apply(&event, &meta);

                        if matches!(event, Event::TxInvalid { .. } | Event::TxValid { .. })
                            && let Err(error) = self.hydra_channel.send(event.clone())
                        {
                            debug!(?error, "failed to send event to internal trp hydra channel");
                        }
                    }

                    Err(error) => {
                        #[derive(Deserialize)]
                        struct TagOnly {
                            tag: Option<String>,
                        }

                        let tag = serde_json::from_str::<TagOnly>(text)
                            .ok()
                            .and_then(|x| x.tag)
                            .unwrap_or_else(|| "<missing>".to_string());

                        info!(%tag, "unhandled Hydra event");
                        debug!(payload = %text, ?error, "Hydra event not supported")
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        };

        let cancellation = async {
            cancellation_token.cancelled().await;
            info!("gracefully shuting down hydra");

            Ok::<(), anyhow::Error>(())
        };

        tokio::select! {
            result = message_processing => {
                result?;
                info!("WebSocket message processing completed");
            }
            result = cancellation => {
                result?;
                info!("Cancellation requested, WebSocket shutting down");
            }
        }

        Ok(())
    }

    pub async fn submit(&self, hydra_message: HydraMessage) -> anyhow::Result<()> {
        let mut sink = self.sink.lock().await;
        let message_bytes = serde_json::to_vec(&hydra_message)?;
        let message = Message::binary(message_bytes);
        sink.send(message)
            .await
            .context("failed to send message to hydra head")?;
        Ok(())
    }

    pub async fn check_health(&self) -> bool {
        let mut sink = self.sink.lock().await;
        let result = sink.send(Message::Ping(Vec::new().into())).await;
        result.is_ok()
    }

    pub async fn get_pparams(&self) -> anyhow::Result<PParams> {
        let client = reqwest::Client::new();

        let req = client
            .get(format!("{}/protocol-parameters", self.config.http_url))
            .build()
            .unwrap();

        let res = client
            .execute(req)
            .await
            .context("fetching http pparams endpoint")?;

        let hydra_pparams = res
            .json::<HydraPParams>()
            .await
            .context("decoding pparams")?;

        let pparams = hydra_pparams.to_tx3_pparams(self.config.network);

        Ok(pparams)
    }

    pub async fn get_progress(&self) -> Progress {
        self.state.read().await.progress.clone()
    }

    pub async fn read_utxos(&self) -> UtxoSnapshot<'_> {
        UtxoSnapshot(self.state.read().await)
    }
}

impl HydraPParams {
    pub fn to_tx3_pparams(&self, network: u8) -> PParams {
        PParams {
            network: network.try_into().unwrap(),
            min_fee_coefficient: self.tx_fee_per_byte,
            min_fee_constant: self.tx_fee_fixed,
            coins_per_utxo_byte: self.utxo_cost_per_byte,
            cost_models: self
                .cost_models
                .clone()
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct Config {
    network: u8,
    ws_url: String,
    http_url: String,
}
