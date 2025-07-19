use std::collections::{HashMap, HashSet};
use tracing::debug;

use crate::hydra::{
    HydraLedger,
    data::{TxID, Utxo},
};

use super::utxos::into_tx3_utxo;

enum Subset {
    All,
    Specific(HashSet<TxID>),
}

impl Subset {
    #[allow(dead_code)]
    fn union(a: Self, b: Self) -> Self {
        match (a, b) {
            (Self::All, _) => Self::All,
            (_, Self::All) => Self::All,
            (Self::Specific(s1), Self::Specific(s2)) => {
                Self::Specific(s1.union(&s2).cloned().collect())
            }
        }
    }

    fn intersection(a: Self, b: Self) -> Self {
        match (a, b) {
            (Self::All, x) => x,
            (x, Self::All) => x,
            (Self::Specific(s1), Self::Specific(s2)) => {
                Self::Specific(s1.intersection(&s2).cloned().collect())
            }
        }
    }

    fn intersection_of_all<const N: usize>(subsets: [Self; N]) -> Self {
        let mut result = Subset::All;

        for subset in subsets {
            result = Self::intersection(result, subset);
        }

        result
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::All => false,
            Self::Specific(s) => s.is_empty(),
        }
    }
}

fn utxo_includes_custom_asset(
    utxo: &Utxo,
    expected: &tx3_lang::ir::AssetExpr,
) -> Result<bool, tx3_cardano::Error> {
    let policy = tx3_cardano::coercion::expr_into_bytes(&expected.policy)?;
    let policy_hex = hex::encode(policy.as_slice());

    let assets = utxo.value.assets_by_policy(&policy_hex);

    if assets.is_empty() {
        return Ok(false);
    }

    let name = tx3_cardano::coercion::expr_into_bytes(&expected.asset_name)?;
    let name_hex = hex::encode(name.as_slice());

    let Some(asset) = assets.get(&name_hex) else {
        return Ok(false);
    };

    let amount = tx3_cardano::coercion::expr_into_number(&expected.amount)?;

    Ok(*asset as i128 >= amount)
}

fn utxo_includes_lovelace_amount(
    utxo: &Utxo,
    amount: &tx3_lang::ir::Expression,
) -> Result<bool, tx3_cardano::Error> {
    let expected = tx3_cardano::coercion::expr_into_number(amount)?;
    Ok(utxo.value.lovelace() >= expected)
}

fn utxo_matches_min_amount(
    utxo: &Utxo,
    min_amount: &tx3_lang::ir::Expression,
) -> Result<bool, tx3_cardano::Error> {
    let expected = tx3_cardano::coercion::expr_into_assets(min_amount)?;

    let lovelace_ok = expected
        .iter()
        .filter(|x| x.policy.is_none())
        .map(|asset| utxo_includes_lovelace_amount(utxo, &asset.amount))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .all(|x| *x);

    let custom_ok = expected
        .iter()
        .filter(|x| !x.policy.is_none())
        .map(|asset| utxo_includes_custom_asset(utxo, asset))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .all(|x| *x);

    Ok(lovelace_ok && custom_ok)
}

fn utxo_matches(
    utxo: &Utxo,
    criteria: &tx3_lang::ir::InputQuery,
) -> Result<bool, tx3_cardano::Error> {
    let min_amount_check = if let Some(min_amount) = &criteria.min_amount.as_option() {
        utxo_matches_min_amount(utxo, min_amount)?
    } else {
        // if there is no min amount requirement, then the utxo matches
        true
    };

    Ok(min_amount_check)
}

fn pick_first_utxo_match(
    utxos: HashMap<TxID, Utxo>,
    criteria: &tx3_lang::ir::InputQuery,
) -> Result<Option<tx3_lang::Utxo>, tx3_cardano::Error> {
    for (tx_id, utxo) in utxos {
        if utxo_matches(&utxo, criteria)? {
            let mapped = into_tx3_utxo((tx_id, utxo))
                .map_err(|err| tx3_cardano::Error::LedgerInternalError(err.to_string()))?;
            return Ok(Some(mapped));
        }
    }

    Ok(None)
}

const MAX_SEARCH_SPACE_SIZE: usize = 50;

pub struct InputSelector {
    ledger: HydraLedger,
    network: tx3_cardano::Network,
}

impl InputSelector {
    pub fn new(ledger: HydraLedger, network: tx3_cardano::Network) -> Self {
        Self { ledger, network }
    }

    fn narrow_by_address(
        &self,
        expr: &tx3_lang::ir::Expression,
    ) -> Result<Subset, tx3_cardano::Error> {
        let address = tx3_cardano::coercion::expr_into_address(expr, self.network)?.to_vec();

        let utxos = self
            .ledger
            .get_utxo_by_address(address)
            .map_err(|error| tx3_cardano::Error::LedgerInternalError(error.to_string()))?;

        Ok(Subset::Specific(utxos.into_iter().collect()))
    }

    fn narrow_by_asset_presence(
        &self,
        expr: &tx3_lang::ir::AssetExpr,
    ) -> Result<Subset, tx3_cardano::Error> {
        let amount = tx3_cardano::coercion::expr_into_number(&expr.amount)?;

        // skip filtering if required amount is 0 since it's not adding any constraints
        if amount == 0 {
            return Ok(Subset::All);
        }

        // skip filtering lovelace since it's not an custom asset
        if expr.policy.is_none() {
            return Ok(Subset::All);
        }

        let policy_bytes = tx3_cardano::coercion::expr_into_bytes(&expr.policy)?;
        let name_bytes = tx3_cardano::coercion::expr_into_bytes(&expr.asset_name)?;

        let utxos = self
            .ledger
            .get_utxo_by_asset(policy_bytes.as_slice(), name_bytes.as_slice());

        Ok(Subset::Specific(utxos.into_iter().collect()))
    }

    fn narrow_by_multi_asset_presence(
        &self,
        expr: &tx3_lang::ir::Expression,
    ) -> Result<Subset, tx3_cardano::Error> {
        let assets = tx3_cardano::coercion::expr_into_assets(expr)?;

        let mut matches = Subset::All;

        for asset in assets {
            let next = self.narrow_by_asset_presence(&asset)?;
            matches = Subset::intersection(matches, next);
        }

        Ok(matches)
    }

    fn narrow_by_ref(&self, expr: &tx3_lang::ir::Expression) -> Result<Subset, tx3_cardano::Error> {
        let refs = tx3_cardano::coercion::expr_into_utxo_refs(expr)?;

        let refs: Vec<String> = refs
            .iter()
            .map(|r| {
                let tx_hash = hex::encode(&r.txid);
                let tx_index = r.index;
                format!("{tx_hash}#{tx_index}")
            })
            .collect();

        let utxos = self.ledger.get_utxo_by_refs(refs);

        Ok(Subset::Specific(utxos.into_iter().collect()))
    }

    fn narrow_search_space(
        &self,
        criteria: &tx3_lang::ir::InputQuery,
    ) -> Result<Subset, tx3_cardano::Error> {
        let matching_address = if let Some(address) = &criteria.address.as_option() {
            self.narrow_by_address(address)?
        } else {
            Subset::All
        };

        if matching_address.is_empty() {
            debug!("matching address is empty");
        }

        let matching_assets = if let Some(min_amount) = &criteria.min_amount.as_option() {
            self.narrow_by_multi_asset_presence(min_amount)?
        } else {
            Subset::All
        };

        if matching_assets.is_empty() {
            debug!("matching assets is empty");
        }

        let matching_refs = if let Some(refs) = &criteria.r#ref.as_option() {
            self.narrow_by_ref(refs)?
        } else {
            Subset::All
        };

        if matching_refs.is_empty() {
            debug!("matching refs is empty");
        }

        Ok(Subset::intersection_of_all([
            matching_address,
            matching_assets,
            matching_refs,
        ]))
    }

    pub fn select(
        &self,
        criteria: &tx3_lang::ir::InputQuery,
    ) -> Result<tx3_lang::UtxoSet, tx3_cardano::Error> {
        let search_space = self.narrow_search_space(criteria)?;

        let refs = match search_space {
            Subset::Specific(refs) if refs.len() <= MAX_SEARCH_SPACE_SIZE => refs,
            Subset::Specific(_) => return Err(tx3_cardano::Error::InputQueryTooBroad),
            Subset::All => return Err(tx3_cardano::Error::InputQueryTooBroad),
        };

        let utxos = self.ledger.get_utxos(refs.into_iter().collect());

        let matched = pick_first_utxo_match(utxos, criteria)?;

        if let Some(utxo) = matched {
            Ok(vec![utxo].into_iter().collect())
        } else {
            Ok(tx3_lang::UtxoSet::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use tx3_cardano::pallas;

    use super::*;

    fn setup_dummy_ledger() -> HydraLedger {
        let utxos =
            serde_json::from_str::<HashMap<TxID, Utxo>>(include_str!("test_data/utxos.json"))
                .unwrap();

        HydraLedger {
            utxos,
            network: 0,
            http_url: "".to_string(),
        }
    }

    fn bech32_address_expr(bech32: &str) -> tx3_lang::ir::Expression {
        let address = pallas::ledger::addresses::Address::from_bech32(bech32).unwrap();
        tx3_lang::ir::Expression::Address(address.to_vec())
    }

    fn custom_asset_expr(policy: &str, name: &str, amount: i128) -> tx3_lang::ir::AssetExpr {
        tx3_lang::ir::AssetExpr {
            policy: tx3_lang::ir::Expression::Bytes(hex::decode(policy).unwrap()),
            asset_name: tx3_lang::ir::Expression::Bytes(hex::decode(name).unwrap()),
            amount: tx3_lang::ir::Expression::Number(amount),
        }
    }

    fn lovelace_expr(amount: i128) -> tx3_lang::ir::AssetExpr {
        tx3_lang::ir::AssetExpr {
            policy: tx3_lang::ir::Expression::None,
            asset_name: tx3_lang::ir::Expression::None,
            amount: tx3_lang::ir::Expression::Number(amount),
        }
    }

    #[test]
    fn test_utxo_match_asset_presence() {
        let ledger = setup_dummy_ledger();
        let selector = InputSelector::new(ledger, tx3_cardano::Network::Testnet);

        let criteria = tx3_lang::ir::InputQuery {
            address: tx3_lang::ir::Expression::None,
            min_amount: tx3_lang::ir::Expression::Assets(vec![custom_asset_expr(
                "954fe5769e9eb8dad54c99f8d62015c813c24f229a4d98dbf05c28b9",
                "434f494e",
                1,
            )]),
            r#ref: tx3_lang::ir::Expression::None,
        };

        let result = selector.select(&criteria).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_utxo_match_address_and_asset_presence() {
        let ledger = setup_dummy_ledger();
        let selector = InputSelector::new(ledger, tx3_cardano::Network::Testnet);

        let criteria = tx3_lang::ir::InputQuery {
            address: bech32_address_expr(
                "addr_test1vp7f4380zv203gjqscn5ls4j6s0v976nnqdhds5n78ty6hqu9e072",
            ),
            min_amount: tx3_lang::ir::Expression::Assets(vec![custom_asset_expr(
                "954fe5769e9eb8dad54c99f8d62015c813c24f229a4d98dbf05c28b9",
                "434f494e",
                1,
            )]),
            r#ref: tx3_lang::ir::Expression::None,
        };

        let result = selector.select(&criteria).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_utxo_match_lovelace_amount() {
        let ledger = setup_dummy_ledger();
        let selector = InputSelector::new(ledger, tx3_cardano::Network::Testnet);

        let criteria = tx3_lang::ir::InputQuery {
            address: bech32_address_expr(
                "addr_test1vp7f4380zv203gjqscn5ls4j6s0v976nnqdhds5n78ty6hqu9e072",
            ),
            min_amount: tx3_lang::ir::Expression::Assets(vec![lovelace_expr(1_000_000_000)]),
            r#ref: tx3_lang::ir::Expression::None,
        };

        let result = selector.select(&criteria).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_utxo_excludes_lower_amount() {
        let ledger = setup_dummy_ledger();
        let selector = InputSelector::new(ledger, tx3_cardano::Network::Testnet);

        let criteria = tx3_lang::ir::InputQuery {
            address: bech32_address_expr(
                "addr_test1vp7f4380zv203gjqscn5ls4j6s0v976nnqdhds5n78ty6hqu9e072",
            ),
            min_amount: tx3_lang::ir::Expression::Assets(vec![lovelace_expr(1_000_000_001)]),
            r#ref: tx3_lang::ir::Expression::None,
        };

        let result = selector.select(&criteria).unwrap();

        assert_eq!(result.len(), 0);
    }
}
