use std::collections::HashMap;

use event::{Event, HeadStatus, TxID, UtxoEntry};
use futures_util::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

mod event;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub struct HydraAdapter {
    snapshot: RwLock<HashMap<TxID, UtxoEntry>>,
    head_status: RwLock<HeadStatus>,
    stream: Mutex<SplitStream<WsStream>>,
    sink: SplitSink<WsStream, Message>,
}

impl HydraAdapter {
    pub async fn try_new(config: Config) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(&config.url).await?;
        info!("Hydra ws handshake has been successfully completed");

        let (write, read) = ws_stream.split();

        let snapshot = RwLock::default();
        let stream = Mutex::new(read);
        let sink = write;
        let head_status = RwLock::new(HeadStatus::Closed);

        Ok(Self {
            snapshot,
            stream,
            sink,
            head_status,
        })
    }

    pub async fn utxos(&self) -> HashMap<TxID, UtxoEntry> {
        self.snapshot.read().await.clone()
    }

    pub async fn run(&self, cancellation_token: CancellationToken) -> anyhow::Result<()> {
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
                            *self.snapshot.write().await = snapshot.utxo;
                        }
                        Event::Bootstrap { head_status, utxo } => {
                            *self.snapshot.write().await = utxo;
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
}

// Implement tx3 ledger adapter

#[derive(Deserialize, Clone)]
pub struct Config {
    url: String,
}
