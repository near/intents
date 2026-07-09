use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use defuse_sep53::Sep53;
use near_sdk::{CryptoHash, serde::de::DeserializeOwned, serde_json};
use serde::{Deserialize, Serialize};

use crate::payload::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSep53Payload {
    pub payload: String,

    pub public_key: Ed25519PublicKey,

    pub signature: Ed25519Signature,
}

impl Payload for SignedSep53Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        Sep53::prehash(&self.payload)
    }
}

impl SignedPayload for SignedSep53Payload {
    type PublicKey = Ed25519PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Sep53::verify(
            &self.public_key.try_into().ok()?,
            &self.payload,
            &self.signature.into(),
        )
        .then_some(&self.public_key)
        .copied()
    }
}

impl<T> ExtractDefusePayload<T> for SignedSep53Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}
