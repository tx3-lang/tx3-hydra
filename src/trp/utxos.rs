use anyhow::Context;
use tx3_cardano::pallas::{
    codec::utils::KeyValuePairs,
    ledger::{
        addresses::Address,
        primitives::{BigInt, Constr, PlutusData},
    },
};
use tx3_lang::ir::{Expression, StructExpr};

use crate::hydra::data::{TxID, Utxo};

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

    let datum = match (&utxo.inline_datum, &utxo.datum, &utxo.inline_datum_raw) {
        (Some(inline_datum), None, None) => {
            let plutus_data = serde_json::from_value::<PlutusData>(inline_datum.clone())
                .context("failed to decode hydra utxo inline datum")?;
            Some(map_datum(&plutus_data))
        }
        (None, Some(datum), None) => {
            let datum = hex::decode(datum).context("failed to decode hydra utxo hex datum")?;
            let plutus_data = serde_json::from_slice::<PlutusData>(&datum)
                .context("failed to decode hydra utxo datum")?;
            Some(map_datum(&plutus_data))
        }
        (None, None, Some(datum_raw)) => {
            let datum =
                hex::decode(datum_raw).context("failed to decode hydra utxo hex cbor datum raw")?;

            Some(Expression::Bytes(datum))
        }
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
        // TODO: implement reference script expression
        script: None,
    };

    Ok(utxo)
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
