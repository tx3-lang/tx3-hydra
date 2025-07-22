use std::collections::HashMap;

use anyhow::Context;
use tx3_cardano::pallas::{
    codec::utils::KeyValuePairs,
    ledger::{
        addresses::Address,
        primitives::{BigInt, Constr, PlutusData},
    },
};
use tx3_lang::{
    CanonicalAssets, UtxoRef,
    ir::{Expression, StructExpr},
};

use crate::hydra::{
    self,
    model::{AssetValue, Utxo},
};

fn map_policy_assets(policy: &str, assets: &HashMap<String, u64>) -> tx3_lang::CanonicalAssets {
    let init = tx3_lang::CanonicalAssets::empty();

    let policy_id = hex::decode(policy).unwrap();

    let all = assets
        .iter()
        .map(|(asset_name, amount)| {
            let asset_name = hex::decode(asset_name).unwrap();
            CanonicalAssets::from_defined_asset(&policy_id, &asset_name, *amount as i128)
        })
        .fold(init, |acc, x| acc + x);

    all
}

fn map_assets(value: &hydra::model::Value) -> tx3_lang::CanonicalAssets {
    let init = tx3_lang::CanonicalAssets::empty();

    let out = value
        .assets
        .iter()
        .map(|(policy, assets)| match assets {
            AssetValue::Lovelace(amount) => CanonicalAssets::from_naked_amount(*amount as i128),
            AssetValue::Multi(assets) => map_policy_assets(policy, assets),
        })
        .fold(init, |acc, x| acc + x);

    out
}

fn map_big_int(x: &BigInt) -> Expression {
    match x {
        BigInt::Int(x) => Expression::Number((*x).into()),
        BigInt::BigUInt(bounded_bytes) => {
            // Convert bytes to big-endian integer
            let mut result = 0i128;
            for &byte in bounded_bytes.iter() {
                result = (result << 8) | (byte as i128);
            }
            Expression::Number(result)
        }
        BigInt::BigNInt(bounded_bytes) => {
            // Convert bytes to big-endian integer and negate
            let mut result = 0i128;
            for &byte in bounded_bytes.iter() {
                result = (result << 8) | (byte as i128);
            }
            Expression::Number(-result)
        }
    }
}

fn map_constr(x: &Constr<PlutusData>) -> Expression {
    Expression::Struct(StructExpr {
        constructor: x.constructor_value().unwrap_or_default() as usize,
        fields: x.fields.iter().map(map_datum).collect(),
    })
}

fn map_array(x: &[PlutusData]) -> Expression {
    Expression::List(x.iter().map(map_datum).collect())
}

fn map_map(x: &KeyValuePairs<PlutusData, PlutusData>) -> Expression {
    Expression::List(
        x.iter()
            .map(|(k, v)| Expression::List(vec![map_datum(k), map_datum(v)]))
            .collect(),
    )
}

fn map_datum(datum: &PlutusData) -> Expression {
    match datum {
        PlutusData::Constr(x) => map_constr(x),
        PlutusData::Map(x) => map_map(x),
        PlutusData::BigInt(x) => map_big_int(x),
        PlutusData::BoundedBytes(x) => Expression::Bytes(x.to_vec()),
        PlutusData::Array(x) => map_array(x),
    }
}

fn pick_utxo_datum(utxo: &Utxo) -> Result<Option<Expression>, anyhow::Error> {
    if let Some(inline) = &utxo.inline_datum {
        let plutus_data = serde_json::from_value::<PlutusData>(inline.clone())
            .context("failed to decode hydra utxo inline datum")?;

        return Ok(Some(map_datum(&plutus_data)));
    }

    if let Some(datum) = &utxo.datum {
        let datum = hex::decode(datum).context("failed to decode hydra utxo hex datum")?;

        let plutus_data = serde_json::from_slice::<PlutusData>(&datum)
            .context("failed to decode hydra utxo datum")?;

        return Ok(Some(map_datum(&plutus_data)));
    }

    if let Some(datum_raw) = &utxo.inline_datum_raw {
        let datum =
            hex::decode(datum_raw).context("failed to decode hydra utxo hex cbor datum raw")?;

        return Ok(Some(Expression::Bytes(datum)));
    }

    Ok(None)
}

pub fn into_tx3_utxo(ref_: UtxoRef, utxo: &Utxo) -> anyhow::Result<tx3_lang::Utxo> {
    let address =
        Address::from_bech32(&utxo.address).context("failed to decode hydra utxo address")?;

    let datum = pick_utxo_datum(utxo)?;

    let assets = map_assets(&utxo.value);

    let utxo = tx3_lang::Utxo {
        address: address.to_vec(),
        r#ref: ref_,
        datum,
        assets,
        // TODO: implement reference script expression
        script: None,
    };

    Ok(utxo)
}
