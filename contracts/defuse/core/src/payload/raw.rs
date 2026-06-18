use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, VerifiableCurve, serde::AsCurve};
use defuse_digest::{Digest, Sha256};
use near_sdk::{near, serde::de::DeserializeOwned, serde_json};

use super::ExtractDefusePayload;

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
        Sha256::digest(self.payload.as_bytes()).into()
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
