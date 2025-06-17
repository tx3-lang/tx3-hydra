use std::collections::HashMap;

use anyhow::Context;
use data::{Event, HeadStatus, HydraMessage, HydraPParams, TxID, Utxo};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tx3_cardano::PParams;

pub mod data;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub struct HydraAdapter {
    pub ledger: RwLock<HydraLedger>,
    head_status: RwLock<HeadStatus>,
    stream: Mutex<SplitStream<WsStream>>,
    sink: Mutex<SplitSink<WsStream, Message>>,
}

impl HydraAdapter {
    pub async fn try_new(config: Config) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(&config.ws_url).await?;
        info!("Hydra ws handshake has been successfully completed");

        let (write, read) = ws_stream.split();

        let ledger = RwLock::new(HydraLedger::new(config));
        let stream = Mutex::new(read);
        let sink = Mutex::new(write);
        let head_status = RwLock::new(HeadStatus::Closed);

        Ok(Self {
            ledger,
            stream,
            sink,
            head_status,
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

                match serde_json::from_str::<Event>(message.to_text().unwrap()) {
                    Ok(event) => match event {
                        Event::Snapshot { snapshot } => {
                            self.ledger.write().await.update(snapshot.utxo);
                        }
                        Event::Bootstrap { head_status, utxo } => {
                            self.ledger.write().await.update(utxo);
                            *self.head_status.write().await = head_status;
                        }
                    },

                    Err(_) => {
                        let message = message.to_text().unwrap();
                        warn!(?message, "Hydra event not supported")
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
}

#[derive(Debug, Clone, Default)]
pub struct HydraLedger {
    pub utxos: HashMap<TxID, Utxo>,
    pub network: u8,
    pub http_url: String,
}
impl HydraLedger {
    pub fn new(config: Config) -> Self {
        Self {
            http_url: config.http_url,
            network: config.network,
            utxos: Default::default(),
        }
    }

    pub fn update(&mut self, utxo_snapshot: HashMap<TxID, Utxo>) {
        self.utxos = utxo_snapshot;
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
