use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

/// Transaction Hash # Index
pub type TxID = String;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "tag")]
pub enum Event {
    Greetings {
        #[serde(rename = "headStatus")]
        head_status: HeadStatus,

        #[serde(rename = "snapshotUtxo", default)]
        snapshot: HashMap<TxID, Utxo>,
    },
    SnapshotConfirmed {
        snapshot: Snapshot,
    },
    HeadIsOpen {
        #[serde(rename = "utxo", default)]
        snapshot: HashMap<TxID, Utxo>,
    },
    CommitApproved {
        #[serde(rename = "utxoToCommit")]
        utxo_to_commit: HashMap<TxID, Utxo>,
    },
    CommitRecovered {
        #[serde(rename = "recoveredUTxO")]
        recovered_utxo: HashMap<TxID, Utxo>,
        #[serde(rename = "recoveredTxId")]
        recovered_tx_id: String,
    },
    CommitRecorded {
        #[serde(rename = "pendingDeposit")]
        pending_deposit: String,
        #[serde(rename = "utxoToCommit")]
        utxo_to_commit: HashMap<TxID, Utxo>,
    },
    DepositActivated {
        #[serde(rename = "depositTxId", alias = "theDeposit")]
        deposit_tx_id: String,
    },
    DepositExpired {
        #[serde(rename = "depositTxId", alias = "theDeposit")]
        deposit_tx_id: String,
    },
    CommitFinalized {
        #[serde(rename = "depositTxId", alias = "theDeposit")]
        deposit_tx_id: String,
    },
    TxValid {
        #[serde(alias = "transactionId")]
        tx_id: String,
    },
    TxInvalid {
        transaction: Transaction,

        #[serde(alias = "validationError")]
        validation_error: ValidationError,
    },
}

#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum HeadStatus {
    #[default]
    Idle,
    Initializing,
    Open,
    Closed,
    FanoutPossible,
    Final,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EventMeta {
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub timestamp: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Snapshot {
    pub utxo: HashMap<TxID, Utxo>,
    #[serde(rename = "utxoToCommit", default)]
    pub utxo_to_commit: Option<HashMap<TxID, Utxo>>,
}

impl Snapshot {
    pub fn full_utxo(self) -> HashMap<TxID, Utxo> {
        let mut utxo = self.utxo;

        if let Some(utxo_to_commit) = self.utxo_to_commit {
            utxo.extend(utxo_to_commit);
        }

        utxo
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HydraPParamsPlutusVersion {
    PlutusV1,
    PlutusV2,
    PlutusV3,
}
impl From<HydraPParamsPlutusVersion> for u8 {
    fn from(value: HydraPParamsPlutusVersion) -> Self {
        match value {
            HydraPParamsPlutusVersion::PlutusV1 => 0,
            HydraPParamsPlutusVersion::PlutusV2 => 1,
            HydraPParamsPlutusVersion::PlutusV3 => 2,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(alias = "txId")]
    pub tx_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ValidationError {
    pub reason: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HydraPParams {
    #[serde(rename = "txFeePerByte")]
    pub tx_fee_per_byte: u64,

    #[serde(rename = "txFeeFixed")]
    pub tx_fee_fixed: u64,

    #[serde(rename = "utxoCostPerByte")]
    pub utxo_cost_per_byte: u64,

    #[serde(rename = "costModels")]
    pub cost_models: HashMap<HydraPParamsPlutusVersion, Vec<i64>>,
}

/// Hydra head utxo data model
#[derive(Deserialize, Debug, Clone)]
pub struct Utxo {
    /// A bech-32 encoded Cardano address
    pub address: String,

    /// Base16 encoding
    pub datum: Option<String>,

    /// Base16 encoding
    #[allow(dead_code)]
    pub datumhash: Option<String>,

    #[serde(rename = "inlineDatum")]
    #[allow(dead_code)]
    pub inline_datum: Option<serde_json::Value>,

    /// Base16 encoding
    #[serde(rename = "inlineDatumhash")]
    #[allow(dead_code)]
    pub inline_datum_hash: Option<String>,

    /// The base16-encoding of the CBOR encoding of some binary data
    #[serde(rename = "inlineDatumRaw")]
    pub inline_datum_raw: Option<String>,

    #[serde(rename = "referenceScript")]
    #[allow(dead_code)]
    pub reference_script: Option<ReferenceScript>,

    pub value: Value,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ReferenceScript {
    /// Base16 encoding
    #[serde(rename = "cborHex")]
    pub cbor_hex: String,

    pub description: String,

    /// Types available: SimpleScript, PlutusScriptV1, PlutusScriptV2, PlutusScriptV3
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AssetValue {
    Lovelace(u64),
    Multi(HashMap<String, u64>),
}

/// Map of asset IDs to amounts
#[derive(Deserialize, Debug, Clone)]
pub struct Value {
    #[serde(flatten)]
    pub assets: HashMap<String, AssetValue>,
}

impl Value {
    pub fn assets_by_policy(&self, policy_hex: &str) -> HashMap<String, u64> {
        let Some(policy_value) = self.assets.get(policy_hex) else {
            return HashMap::new();
        };

        match policy_value {
            AssetValue::Lovelace(_) => HashMap::new(),
            AssetValue::Multi(map) => map.clone(),
        }
    }
}

/// Tags accepted by hydra Websocket
#[derive(Debug, Clone)]
pub enum HydraMessage {
    NewTx(NewTx),
}

impl Serialize for HydraMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            HydraMessage::NewTx(tx) => {
                let mut state = serializer.serialize_struct("Message", 2)?;
                state.serialize_field("tag", "NewTx")?;
                state.serialize_field("transaction", tx)?;
                state.end()
            }
        }
    }
}

/// Submit new tx using Websocket
#[derive(Serialize, Debug, Clone)]
pub struct NewTx {
    pub r#type: String,
    pub description: String,
    #[serde(rename = "cborHex")]
    pub cbor_hex: String,
}

impl NewTx {
    pub fn new(cbor: Vec<u8>) -> Self {
        let r#type = String::from("Tx ConwayEra");
        let description = String::from("Tx3 Transaction");
        let cbor_hex = hex::encode(cbor);

        Self {
            r#type,
            description,
            cbor_hex,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str) -> (Event, EventMeta) {
        let text = match name {
            "greetings_idle_2x" => include_str!("test_data/greetings_idle_2x.json"),
            "greetings_open" => include_str!("test_data/greetings_open.json"),
            "head_is_open_2x" => include_str!("test_data/head_is_open_2x.json"),
            "head_is_open_legacy" => include_str!("test_data/head_is_open_legacy.json"),
            "snapshot_confirmed_legacy" => include_str!("test_data/snapshot_confirmed_legacy.json"),
            "snapshot_confirmed_null_commit" => {
                include_str!("test_data/snapshot_confirmed_null_commit.json")
            }
            "snapshot_confirmed_with_commit" => {
                include_str!("test_data/snapshot_confirmed_with_commit.json")
            }
            "commit_recorded" => include_str!("test_data/commit_recorded.json"),
            "deposit_activated" => include_str!("test_data/deposit_activated.json"),
            "deposit_expired" => include_str!("test_data/deposit_expired.json"),
            "commit_approved" => include_str!("test_data/commit_approved.json"),
            "commit_recovered" => include_str!("test_data/commit_recovered.json"),
            "commit_finalized_021" => include_str!("test_data/commit_finalized_021.json"),
            "commit_finalized_020" => include_str!("test_data/commit_finalized_020.json"),
            "tx_valid" => include_str!("test_data/tx_valid.json"),
            "tx_invalid" => include_str!("test_data/tx_invalid.json"),
            _ => unreachable!(),
        };

        (
            serde_json::from_str(text).unwrap(),
            serde_json::from_str(text).unwrap(),
        )
    }

    #[test]
    fn fixtures_deserialize_to_expected_variants() {
        assert!(
            matches!(parse("greetings_idle_2x").0, Event::Greetings { snapshot, .. } if snapshot.is_empty())
        );
        assert!(
            matches!(parse("greetings_open").0, Event::Greetings { head_status: HeadStatus::Open, snapshot } if snapshot.len() == 1)
        );
        assert!(
            matches!(parse("head_is_open_2x").0, Event::HeadIsOpen { snapshot } if snapshot.is_empty())
        );
        assert!(
            matches!(parse("head_is_open_legacy").0, Event::HeadIsOpen { snapshot } if snapshot.len() == 1)
        );
        assert!(
            matches!(parse("snapshot_confirmed_legacy").0, Event::SnapshotConfirmed { snapshot } if snapshot.utxo.len() == 1 && snapshot.utxo_to_commit.is_none())
        );
        assert!(
            matches!(parse("snapshot_confirmed_null_commit").0, Event::SnapshotConfirmed { snapshot } if snapshot.utxo_to_commit.is_none())
        );
        assert!(
            matches!(parse("snapshot_confirmed_with_commit").0, Event::SnapshotConfirmed { snapshot } if snapshot.clone().full_utxo().len() == 2)
        );
        assert!(
            matches!(parse("commit_recorded").0, Event::CommitRecorded { pending_deposit, utxo_to_commit } if pending_deposit == "deposit-tx" && utxo_to_commit.len() == 1)
        );
        assert!(
            matches!(parse("deposit_activated").0, Event::DepositActivated { deposit_tx_id } if deposit_tx_id == "deposit-tx")
        );
        assert!(
            matches!(parse("deposit_expired").0, Event::DepositExpired { deposit_tx_id } if deposit_tx_id == "deposit-tx")
        );
        assert!(
            matches!(parse("commit_approved").0, Event::CommitApproved { utxo_to_commit } if utxo_to_commit.len() == 1)
        );
        assert!(
            matches!(parse("commit_recovered").0, Event::CommitRecovered { recovered_utxo, recovered_tx_id } if recovered_utxo.len() == 1 && recovered_tx_id == "recover-tx")
        );
        assert!(
            matches!(parse("commit_finalized_021").0, Event::CommitFinalized { deposit_tx_id } if deposit_tx_id == "deposit-tx")
        );
        assert!(
            matches!(parse("commit_finalized_020").0, Event::CommitFinalized { deposit_tx_id } if deposit_tx_id == "deposit-tx")
        );
        assert!(matches!(parse("tx_valid").0, Event::TxValid { tx_id } if tx_id == "valid-tx"));
        assert!(
            matches!(parse("tx_invalid").0, Event::TxInvalid { transaction, validation_error } if transaction.tx_id == "invalid-tx" && validation_error.reason == "bad")
        );
    }

    #[test]
    fn event_meta_parses_from_fixtures() {
        let (_, meta) = parse("commit_approved");

        assert_eq!(meta.seq, 10);
        assert_eq!(meta.timestamp, "2026-01-01T00:00:10Z");
    }
}
