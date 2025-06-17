use std::collections::HashMap;

use serde::Deserialize;

/// Transaction Hash # Index
pub type TxID = String;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Event {
    Bootstrap {
        #[serde(rename = "headStatus")]
        head_status: HeadStatus,

        #[serde(alias = "snapshotUtxo")]
        utxo: HashMap<TxID, Utxo>,
    },
    Snapshot {
        snapshot: Snapshot,
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
            HydraPParamsPlutusVersion::PlutusV1 => 1,
            HydraPParamsPlutusVersion::PlutusV2 => 2,
            HydraPParamsPlutusVersion::PlutusV3 => 3,
        }
    }
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

/// Map of asset IDs to amounts
#[derive(Deserialize, Debug, Clone)]
pub struct Value {
    #[serde(flatten)]
    pub assets: HashMap<String, u64>,
}
