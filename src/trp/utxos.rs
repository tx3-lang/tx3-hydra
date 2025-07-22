use std::collections::HashSet;

use tx3_cardano::pallas::ledger::addresses::Address;
use tx3_lang::{
    UtxoRef, UtxoSet,
    backend::{Error, UtxoPattern, UtxoStore},
};

use crate::hydra::{
    UtxoSnapshot,
    model::{TxID, Utxo},
};

fn parse_txid(txid: &str) -> Option<UtxoRef> {
    let (txid, index) = txid.split_once("#")?;
    let hash = hex::decode(txid).ok()?;
    let index = index.parse::<u32>().ok()?;
    let utxo_ref = UtxoRef::new(&hash, index);

    Some(utxo_ref)
}

impl UtxoSnapshot<'_> {
    pub fn get_utxo_by_address(&self, address: &[u8]) -> Vec<TxID> {
        let address = Address::from_bytes(address).map(|x| x.to_string()).ok();

        let Some(address) = address else {
            return Vec::new();
        };

        self.0
            .iter()
            .filter(|(_, utxo)| utxo.address.eq(&address))
            .map(|(tx_id, _)| tx_id.clone())
            .collect()
    }

    pub fn get_utxo_by_asset_policy(&self, policy: &[u8]) -> Vec<TxID> {
        let policy_hex = hex::encode(policy);

        let utxo_matches = |utxo: &Utxo| {
            let by_policy = utxo.value.assets_by_policy(&policy_hex);
            by_policy.len() > 0
        };

        self.0
            .iter()
            .filter(|(_, utxo)| utxo_matches(utxo))
            .map(|(tx_id, _)| tx_id.clone())
            .collect()
    }

    pub fn get_utxo_by_asset(&self, policy: &[u8], name: &[u8]) -> Vec<TxID> {
        let policy_hex = hex::encode(policy);
        let name_hex = hex::encode(name);

        let utxo_matches = |utxo: &Utxo| {
            let by_policy = utxo.value.assets_by_policy(&policy_hex);
            by_policy.contains_key(&name_hex)
        };

        self.0
            .iter()
            .filter(|(_, utxo)| utxo_matches(utxo))
            .map(|(tx_id, _)| tx_id.clone())
            .collect()
    }
}

impl UtxoStore for UtxoSnapshot<'_> {
    async fn narrow_refs(&self, pattern: UtxoPattern<'_>) -> Result<HashSet<UtxoRef>, Error> {
        let txids = match pattern {
            UtxoPattern::ByAddress(address) => self.get_utxo_by_address(&address),
            UtxoPattern::ByAssetPolicy(policy) => self.get_utxo_by_asset_policy(policy),
            UtxoPattern::ByAsset(policy, name) => self.get_utxo_by_asset(policy, name),
        };

        let refs = txids
            .into_iter()
            .map(|txid| parse_txid(&txid))
            .flatten()
            .collect();

        Ok(refs)
    }

    async fn fetch_utxos(&self, refs: HashSet<UtxoRef>) -> Result<UtxoSet, Error> {
        let mut utxos = HashSet::new();

        for ref_ in refs {
            let txid = format!("{}#{}", hex::encode(&ref_.txid), ref_.index);

            let utxo = self.0.get(&txid).ok_or(Error::UtxoNotFound(ref_.clone()))?;

            let utxo = super::mapping::into_tx3_utxo(ref_, utxo)
                .map_err(|x| Error::StoreError(x.to_string()))?;

            utxos.insert(utxo);
        }

        Ok(utxos)
    }
}
