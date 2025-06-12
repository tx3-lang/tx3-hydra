use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub struct HydraAdapter {
    snapshot: RwLock<Snapshot>,
}

impl HydraAdapter {
    pub fn new() -> Self {
        Self {
            snapshot: RwLock::default(),
        }
    }

    pub async fn run(
        &self,
        config: Config,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<()> {
        let (ws_stream, _) = connect_async(&config.url).await?;
        info!("Hydra ws handshake has been successfully completed");

        let (_write, mut read) = ws_stream.split();

        let message_processing = async {
            while let Some(result) = read.next().await {
                let message = result?;

                if message.is_close() {
                    info!("Received WebSocket close message");
                    break;
                }

                let value = serde_json::from_str::<serde_json::Value>(message.to_text().unwrap())?;
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
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

#[derive(Deserialize, Debug, Default)]
struct Snapshot {
    utxo: HashMap<String, UtxoEntry>,
    // #[serde(rename = "utxoToCommit")]
    // utxo_to_commit: Option<serde_json::Value>,
    // #[serde(rename = "utxoToDecommit")]
    // utxo_to_decommit: Option<serde_json::Value>,
    version: u64,
}

/// Hydra head utxo data model
#[derive(Deserialize, Debug)]
struct UtxoEntry {
    /// A bech-32 encoded Cardano address
    address: String,

    /// Base16 encoding
    datum: Option<String>,

    /// Base16 encoding
    datumhash: Option<String>,

    #[serde(rename = "inlineDatum")]
    inline_datum: Option<serde_json::Value>,

    /// Base16 encoding
    #[serde(rename = "inlineDatumhash")]
    inline_datum_hash: Option<String>,

    /// The base16-encoding of the CBOR encoding of some binary data
    #[serde(rename = "inlineDatumRaw")]
    inline_datum_raw: Option<String>,

    #[serde(rename = "referenceScript")]
    reference_script: Option<ReferenceScript>,

    value: Value,
}

#[derive(Deserialize, Debug)]
struct ReferenceScript {
    /// Base16 encoding
    #[serde(rename = "cborHex")]
    cbor_hex: String,

    description: String,

    /// Types available: SimpleScript, PlutusScriptV1, PlutusScriptV2, PlutusScriptV3
    r#type: String,
}

/// Map of asset IDs to amounts
#[derive(Deserialize, Debug)]
struct Value {
    #[serde(flatten)]
    assets: HashMap<String, u64>,
}
