use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

// TODO: schemars

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignRequest<'a> {
    pub path: Cow<'a, str>,

    #[serde(rename = "payload_v2")]
    pub payload: Payload<'a>,

    pub domain_id: u64,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Payload<'a> {
    /// ECDSA prehash
    Ecdsa(#[serde_as(as = "Hex")] [u8; 32]),
    Eddsa(#[serde_as(as = "Hex")] Cow<'a, [u8]>),
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "scheme")]
pub enum SignResponse {
    Secp256k1(K256Signature),
    Ed25519 {
        #[serde_as(as = "[_; 64]")]
        signature: [u8; 64],
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct K256Signature {
    pub big_r: K256AffinePoint,
    pub s: K256Scalar,
    pub recovery_id: u8,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct K256AffinePoint {
    #[serde_as(as = "Hex")]
    pub affine_point: [u8; 33],
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct K256Scalar {
    #[serde_as(as = "Hex")]
    pub scalar: [u8; 32],
}
