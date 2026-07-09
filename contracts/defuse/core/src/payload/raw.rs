use defuse_crypto::{
    Curve,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_digest::{Digest, sha2::Sha256};
use near_sdk::{serde::de::DeserializeOwned, serde_json};
use serde::{Deserialize, Serialize};

use crate::payload::{Payload, SignedPayload};

use super::ExtractDefusePayload;

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedRawEd25519Payload {
    pub payload: String,

    pub public_key: Ed25519PublicKey,
    pub signature: Ed25519Signature,
}

impl Payload for SignedRawEd25519Payload {
    #[inline]
    fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.payload.as_bytes()).into()
    }
}

impl SignedPayload for SignedRawEd25519Payload {
    type PublicKey = Ed25519PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(
            &self.public_key.try_into().ok()?,
            self.payload.as_bytes(),
            &self.signature.into(),
        )
        .then_some(&self.public_key)
        .copied()
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
