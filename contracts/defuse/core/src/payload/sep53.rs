use crate::payload::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};
use defuse_crypto::{Curve, CurveTypes, Ed25519};
use defuse_sep53::{Sep53Payload, SignedSep53Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl Payload for Sep53Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        near_sdk::env::sha256_array(self.prehash())
    }
}

impl Payload for SignedSep53Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        Payload::hash(&self.payload)
    }
}

impl SignedPayload for SignedSep53Payload {
    type PublicKey = <Ed25519 as CurveTypes>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &Payload::hash(self), &self.public_key)
    }
}

impl<T> ExtractDefusePayload<T> for SignedSep53Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<T> ExtractDefusePayload<T> for Sep53Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}
