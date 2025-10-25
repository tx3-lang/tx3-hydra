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

    

    assets
        .iter()
        .map(|(asset_name, amount)| {
            let asset_name = hex::decode(asset_name).unwrap();
            CanonicalAssets::from_defined_asset(&policy_id, &asset_name, *amount as i128)
        })
        .fold(init, |acc, x| acc + x)
}

fn map_assets(value: &hydra::model::Value) -> tx3_lang::CanonicalAssets {
    let init = tx3_lang::CanonicalAssets::empty();

    

    value
        .assets
        .iter()
        .map(|(policy, assets)| match assets {
            AssetValue::Lovelace(amount) => CanonicalAssets::from_naked_amount(*amount as i128),
            AssetValue::Multi(assets) => map_policy_assets(policy, assets),
        })
        .fold(init, |acc, x| acc + x)
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
    if let Some(datum_raw) = &utxo.inline_datum_raw {
        let plutus_data = plutus_data_from_cbor_hex(datum_raw)?;
        return Ok(Some(map_datum(&plutus_data)));
    }

    if let Some(datum) = &utxo.datum {
        let plutus_data = plutus_data_from_cbor_hex(datum)?;
        return Ok(Some(map_datum(&plutus_data)));
    }

    if let Some(inline) = &utxo.inline_datum {
        let plutus_data = from_hydra_json(inline)
            .context("failed to decode hydra utxo inline datum (json fallback)")?;
        return Ok(Some(map_datum(&plutus_data)));
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

fn plutus_data_from_cbor_hex(hex_str: &str) -> Result<PlutusData, anyhow::Error> {
    let bytes = hex::decode(hex_str)
        .context("failed to decode hydra utxo hex cbor datum raw")?;

    let data: PlutusData =
        minicbor::decode(&bytes).context("failed to CBOR-decode hydra utxo datum raw")?;

    Ok(data)
}

fn from_hydra_json(v: &serde_json::Value) -> anyhow::Result<PlutusData> {
    use tx3_cardano::pallas::ledger::primitives::{PlutusData, Constr, BigInt};

    let obj = v.as_object().context("expected object in hydra datum")?;

    if obj.contains_key("constructor") {
        let constructor = obj["constructor"]
            .as_u64()
            .context("constructor not u64")?;

        let fields_json = obj["fields"]
            .as_array()
            .context("fields not array")?;

        let fields = fields_json
            .iter()
            .map(from_hydra_json)
            .collect::<Result<Vec<_>, _>>()?;

        let constr = Constr {
            tag: constructor as u64,
            any_constructor: None,
            fields,
        };

        return Ok(PlutusData::Constr(constr));
    }

    if let Some(int_v) = obj.get("int") {
        if let Some(n) = int_v.as_i64() {
            return Ok(PlutusData::BigInt(BigInt::Int(n.into())));
        }
    }

    if let Some(bytes_v) = obj.get("bytes") {
        let hex_str = bytes_v
            .as_str()
            .context("bytes not string")?;
        let raw = hex::decode(hex_str)?;
        return Ok(PlutusData::BoundedBytes(raw.into()));
    }

    if let Some(list_v) = obj.get("list") {
        let arr = list_v
            .as_array()
            .context("list not array")?;
        let vals = arr
            .iter()
            .map(from_hydra_json)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(PlutusData::Array(vals));
    }

    if let Some(map_v) = obj.get("map") {
        let arr = map_v
            .as_array()
            .context("map not array")?;

        let mut kvs = KeyValuePairs::default();
        for pair in arr {
            let k = from_hydra_json(&pair["k"])?;
            let val = from_hydra_json(&pair["v"])?;
            kvs.insert(k, val);
        }
        return Ok(PlutusData::Map(kvs));
    }

    Err(anyhow::anyhow!("unrecognized hydra datum json shape"))
}