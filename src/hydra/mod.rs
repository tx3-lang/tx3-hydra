use std::collections::HashMap;

use event::{Event, HeadStatus, HydraPParams, TxID, UtxoEntry};
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
use tx3_cardano::{Error, Ledger, PParams, pallas::ledger::addresses::Address};
use tx3_lang::ir::InputQuery;

mod event;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub struct HydraAdapter {
    pub ledger: RwLock<HydraLedger>,
    head_status: RwLock<HeadStatus>,
    stream: Mutex<SplitStream<WsStream>>,
    sink: SplitSink<WsStream, Message>,
}

impl HydraAdapter {
    pub async fn try_new(config: Config) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(&config.ws_url).await?;
        info!("Hydra ws handshake has been successfully completed");

        let (write, read) = ws_stream.split();

        let ledger = RwLock::new(HydraLedger::new(config));
        let stream = Mutex::new(read);
        let sink = write;
        let head_status = RwLock::new(HeadStatus::Closed);

        Ok(Self {
            ledger,
            stream,
            sink,
            head_status,
        })
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
}

#[derive(Debug, Clone, Default)]
pub struct HydraLedger {
    utxo: HashMap<TxID, UtxoEntry>,
    network: u8,
    http_url: String,
}
impl HydraLedger {
    pub fn new(config: Config) -> Self {
        Self {
            http_url: config.http_url,
            network: config.network,
            utxo: Default::default(),
        }
    }

    pub fn update(&mut self, utxo_snapshot: HashMap<TxID, UtxoEntry>) {
        self.utxo = utxo_snapshot;
    }
}

impl Ledger for HydraLedger {
    async fn get_pparams(&self) -> Result<PParams, Error> {
        let client = reqwest::Client::new();

        let req = client
            .get(format!("{}/protocol-parameters", self.http_url))
            .build()
            .unwrap();

        let res = client.execute(req).await.map_err(|error| {
            error!(?error);
            Error::LedgerInternalError("failed to collect hydra protocol parameters".into())
        })?;

        let hydra_pparams = res.json::<HydraPParams>().await.map_err(|error| {
            error!(?error);
            Error::LedgerInternalError("failed to decode hydra protocol parameters".into())
        })?;

        let pparams = hydra_pparams.to_tx3_pparams(self.network);

        Ok(pparams)
    }

    async fn resolve_input(&self, query: &InputQuery) -> Result<tx3_lang::UtxoSet, Error> {
        let mut utxos = Vec::new();

        for (tx_id, utxo) in self.utxo.iter() {
            let split: Vec<&str> = tx_id.split("#").collect();

            let tx_id_vec = hex::decode(split[0]).map_err(|error| {
                error!(?error);
                Error::LedgerInternalError("failed to decode hydra utxo".into())
            })?;

            let tx_id_index = split[1].parse::<u32>().map_err(|error| {
                error!(?error);
                Error::LedgerInternalError("failed to decode hydra utxo".into())
            })?;

            let utxo_ref = tx3_lang::UtxoRef {
                txid: tx_id_vec,
                index: tx_id_index,
            };

            let address = Address::from_bech32(&utxo.address).map_err(|error| {
                error!(?error);
                Error::LedgerInternalError("failed to decode hydra utxo".into())
            })?;

            let datum = match (&utxo.datum, &utxo.inline_datum, &utxo.inline_datum_raw) {
                (Some(datum), None, None) => todo!(),
                (None, Some(_), None) => todo!(),
                (None, None, Some(_)) => todo!(),
                _ => None,
            };

            let assets = utxo
                .value
                .assets
                .iter()
                .map(|(unit, amount)| {
                    let mut asset_expr = tx3_lang::ir::AssetExpr {
                        amount: tx3_lang::ir::Expression::Number(*amount as i128),
                        policy: tx3_lang::ir::Expression::None,
                        asset_name: tx3_lang::ir::Expression::None,
                    };

                    if unit != "lovelace" {
                        let policy_id_hex = &unit[..56];
                        let policy_id = hex::decode(policy_id_hex).unwrap();
                        asset_expr.policy = tx3_lang::ir::Expression::Bytes(policy_id);

                        let asset_name_hex = &unit[56..];
                        let asset_name = hex::decode(asset_name_hex).unwrap();
                        asset_expr.asset_name = tx3_lang::ir::Expression::Bytes(asset_name);
                    }

                    asset_expr
                })
                .collect();

            let utxo = tx3_lang::Utxo {
                address: address.to_vec(),
                r#ref: utxo_ref,
                datum,
                assets,
                script: None,
            };

            utxos.push(utxo);
        }

        todo!()
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
