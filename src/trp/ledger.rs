use std::collections::HashMap;

use tracing::error;
use tx3_cardano::{Error, Ledger, PParams, pallas::ledger::addresses::Address};
use tx3_lang::ir::InputQuery;

use crate::{
    hydra::{
        self, HydraLedger,
        data::{AssetValue, HydraPParams, TxID, Utxo},
    },
    trp::inputs::InputSelector,
};

impl HydraLedger {
    pub fn get_utxo_by_address(&self, address: Vec<u8>) -> anyhow::Result<Vec<TxID>> {
        let address = Address::from_bytes(&address)?.to_bech32()?;

        let tx_ids = self
            .utxos
            .iter()
            .filter(|(_, utxo)| utxo.address.eq(&address))
            .map(|(tx_id, _)| tx_id.clone())
            .collect();

        Ok(tx_ids)
    }

    pub fn get_utxo_by_asset(&self, unit: Vec<u8>) -> Vec<TxID> {
        let unit_hex = hex::encode(unit);

        self.utxos
            .iter()
            .filter(|(_, utxo)| {
                utxo.value
                    .assets
                    .iter()
                    .any(|(asset, _amount)| asset.eq(&unit_hex))
            })
            .map(|(tx_id, _)| tx_id.clone())
            .collect()
    }

    pub fn get_utxo_by_refs(&self, refs: Vec<TxID>) -> Vec<TxID> {
        self.utxos
            .iter()
            .filter(|(tx_id, _)| refs.contains(tx_id))
            .map(|(tx_id, _)| tx_id.clone())
            .collect()
    }

    pub fn get_utxos(&self, refs: Vec<TxID>) -> HashMap<TxID, Utxo> {
        self.utxos
            .iter()
            .filter(|(tx_id, _)| refs.contains(tx_id))
            .map(|(tx_id, utxo)| (tx_id.clone(), utxo.clone()))
            .collect()
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
        let network = self
            .network
            .try_into()
            .map_err(|_| Error::LedgerInternalError("Invalid network id configuration".into()))?;

        let input_selector = InputSelector::new(self.clone(), network);
        let utxos = input_selector.select(query)?;

        Ok(utxos)
    }
}

impl hydra::data::Value {
    pub fn lovelace(&self) -> i128 {
        match self.assets.get("lovelace") {
            Some(AssetValue::Lovelace(amount)) => *amount as i128,
            _ => 0,
        }
    }

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
