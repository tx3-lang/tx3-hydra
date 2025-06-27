use serde::Deserialize;

pub mod health;
pub mod resolve;
pub mod submit;

#[derive(Deserialize, Debug)]
pub enum Encoding {
    #[serde(rename = "hex")]
    Hex,
    #[serde(rename = "base64")]
    Base64,
}
