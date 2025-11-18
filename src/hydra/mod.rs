use anyhow::Context;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock, RwLockReadGuard, broadcast},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use tx3_cardano::PParams;

pub mod model;

use model::{Event, HeadStatus, HydraMessage, HydraPParams, TxID, Utxo};

pub struct UtxoSnapshot<'a>(pub RwLockReadGuard<'a, HashMap<TxID, Utxo>>);

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub seq: u64,
    pub timestamp: String,
}

pub struct HydraAdapter {
    config: Config,
    progress: RwLock<Progress>,
    utxos: RwLock<HashMap<TxID, Utxo>>,
    head_status: RwLock<HeadStatus>,
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

        let progress = RwLock::new(Progress::default());
        let utxos = RwLock::new(HashMap::new());
        let stream = Mutex::new(read);
        let sink = Mutex::new(write);
        let head_status = RwLock::new(HeadStatus::Closed);

        Ok(Self {
            config,
            progress,
            utxos,
            stream,
            sink,
            head_status,
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

                match serde_json::from_str::<Event>(message.to_text().unwrap()) {
                    Ok(event) => match event {
                        Event::Greetings {
                            head_status,
                            snapshot,
                        } => {
                            info!(utxos = snapshot.len(), "Greetings event");
                            self.update_utxos(snapshot).await;
                            *self.head_status.write().await = head_status;
                        }
                        Event::SnapshotConfirmed {
                            snapshot,
                            seq,
                            timestamp,
                        } => {
                            self.update_utxos(snapshot.utxo).await;
                            self.update_progress(seq, timestamp).await;
                        }
                        Event::HeadIsOpen { snapshot } => {
                            self.update_utxos(snapshot).await;
                            *self.head_status.write().await = HeadStatus::Open;
                        }
                        Event::TxInvalid { .. } | Event::TxValid { .. } => {
                            if let Err(error) = self.hydra_channel.send(event.clone()) {
                                debug!(
                                    ?error,
                                    "failed to send event to internal trp hydra channel"
                                );
                            }
                        }
                    },

                    Err(_) => {
                        let message = message.to_text().unwrap();
                        debug!(?message, "Hydra event not supported")
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
        self.progress.read().await.clone()
    }

    pub async fn update_utxos(&self, utxos: HashMap<TxID, Utxo>) {
        let utxos_len = utxos.len();
        *self.utxos.write().await = utxos;
        info!(utxos = utxos_len, "Snapshot updated");
    }

    pub async fn update_progress(&self, seq: u64, timestamp: String) {
        *self.progress.write().await = Progress { seq, timestamp };
    }

    pub async fn read_utxos(&self) -> UtxoSnapshot {
        UtxoSnapshot(self.utxos.read().await)
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
