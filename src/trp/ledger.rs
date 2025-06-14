use std::collections::HashMap;

use anyhow::Context;
use tracing::error;
use tx3_cardano::{Error, Ledger, PParams, pallas::ledger::addresses::Address};
use tx3_lang::ir::InputQuery;

use crate::{
    hydra::{
        self, HydraLedger,
        data::{HydraPParams, TxID, Utxo},
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
    pub fn coin(&self) -> i128 {
        *self.assets.get("lovelace").unwrap() as i128
    }

    pub fn assets(&self) -> HashMap<String, u64> {
        self.assets
            .clone()
            .into_iter()
            .filter(|(unit, _)| !unit.as_str().eq("lovelace"))
            .collect()
    }
}

pub fn into_tx3_utxo(hydra_utxo: (TxID, Utxo)) -> anyhow::Result<tx3_lang::Utxo> {
    let (tx_id, utxo) = hydra_utxo;
    let split: Vec<&str> = tx_id.split("#").collect();

    let tx_id_vec = hex::decode(split[0]).context("failed to decode hydra utxo tx hash")?;

    let tx_id_index = split[1]
        .parse::<u32>()
        .context("failed to decode hydra utxo tx index")?;

    let utxo_ref = tx3_lang::UtxoRef {
        txid: tx_id_vec,
        index: tx_id_index,
    };

    let address =
        Address::from_bech32(&utxo.address).context("failed to decode hydra utxo address")?;

    let datum = match (&utxo.datum, &utxo.inline_datum, &utxo.inline_datum_raw) {
        (Some(_datum), None, None) => todo!(),
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

    Ok(utxo)
}
