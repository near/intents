use std::borrow::Cow;

use defuse_kdf::crypto::{ed25519::Ed25519PublicKey, secp256k1::Secp256k1UncompressedPublicKey};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

#[near_kit::contract]
pub trait MpcContract {
    fn public_key(&self, args: PublicKeyArgs) -> PublicKey;
    #[call]
    fn sign(&mut self, args: SignArgs<'_>) -> SignResponse;
}

#[derive(Serialize)]
pub struct PublicKeyArgs {
    pub domain_id: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PublicKey {
    Secp256k1(Secp256k1UncompressedPublicKey),
    Ed25519(Ed25519PublicKey),
}

#[derive(Serialize)]
pub struct SignArgs<'a> {
    pub request: SignRequest<'a>,
}

#[derive(Serialize)]
pub struct SignRequest<'a> {
    pub path: Cow<'a, str>,

    #[serde(rename = "payload_v2")]
    pub payload: Payload<'a>,

    pub domain_id: u64,
}

#[serde_as]
#[derive(Serialize)]
pub enum Payload<'a> {
    /// ECDSA prehash
    Ecdsa(#[serde_as(as = "Hex")] [u8; 32]),

    #[allow(clippy::doc_markdown)]
    /// EdDSA payload
    Eddsa(#[serde_as(as = "Hex")] Cow<'a, [u8]>),
}

#[serde_as]
#[derive(Deserialize)]
#[serde(tag = "scheme")]
pub enum SignResponse {
    Secp256k1(K256Signature),
    Ed25519 {
        #[serde_as(as = "[_; 64]")]
        signature: [u8; 64],
    },
}

#[derive(Deserialize)]
pub struct K256Signature {
    pub big_r: K256AffinePoint,
    pub s: K256Scalar,
    pub recovery_id: u8,
}

#[serde_as]
#[derive(Deserialize)]
pub struct K256AffinePoint {
    #[serde_as(as = "Hex")]
    pub affine_point: [u8; 33],
}

#[serde_as]
#[derive(Deserialize)]
pub struct K256Scalar {
    #[serde_as(as = "Hex")]
    pub scalar: [u8; 32],
}
