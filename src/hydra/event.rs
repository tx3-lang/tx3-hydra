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
        utxo: HashMap<TxID, UtxoEntry>,
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
    pub utxo: HashMap<TxID, UtxoEntry>,
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
pub struct UtxoEntry {
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

#[derive(Deserialize, Debug, Clone)]
pub struct ReferenceScript {
    /// Base16 encoding
    #[serde(rename = "cborHex")]
    cbor_hex: String,

    description: String,

    /// Types available: SimpleScript, PlutusScriptV1, PlutusScriptV2, PlutusScriptV3
    r#type: String,
}

/// Map of asset IDs to amounts
#[derive(Deserialize, Debug, Clone)]
pub struct Value {
    #[serde(flatten)]
    assets: HashMap<String, u64>,
}
