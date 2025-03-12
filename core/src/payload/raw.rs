use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use near_sdk::{env, near, serde::de::DeserializeOwned, serde_json};
use serde_with::serde_as;

use super::ExtractDefusePayload;

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct SignedRawEd25519Payload {
    pub payload: String,

    #[serde_as(as = "AsCurve<Ed25519>")]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[serde_as(as = "AsCurve<Ed25519>")]
    pub signature: <Ed25519 as Curve>::Signature,
}

impl Payload for SignedRawEd25519Payload {
    #[inline]
    fn hash(&self) -> [u8; 32] {
        env::sha256_array(self.payload.as_bytes())
    }
}

impl SignedPayload for SignedRawEd25519Payload {
    type PublicKey = <Ed25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, self.payload.as_bytes(), &self.public_key)
    }
}

impl<T> ExtractDefusePayload<T> for SignedRawEd25519Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<super::DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}
