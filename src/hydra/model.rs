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

        #[serde(alias = "snapshotUtxo")]
        snapshot: HashMap<TxID, Utxo>,
    },
    SnapshotConfirmed {
        snapshot: Snapshot,
    },
    HeadIsOpen {
        #[serde(alias = "utxo")]
        snapshot: HashMap<TxID, Utxo>,
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

#[derive(Deserialize, Debug, Clone)]
pub enum HeadStatus {
    Idle,
    Initializing,
    Open,
    Closed,
    FanoutPossible,
    Final,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Snapshot {
    pub utxo: HashMap<TxID, Utxo>,
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
            AssetValue::Lovelace(_) => return HashMap::new(),
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
