use super::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};
use defuse_crypto::{Curve, CurveTypes, Secp256k1};
use defuse_tip191::{SignedTip191Payload, Tip191Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl Payload for Tip191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        near_sdk::env::keccak256_array(self.prehash())
    }
}

impl Payload for SignedTip191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        Payload::hash(&self.payload)
    }
}

impl SignedPayload for SignedTip191Payload {
    type PublicKey = <Secp256k1 as CurveTypes>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        <Secp256k1 as Curve>::verify(&self.signature, &Payload::hash(self), &())
    }
}

impl<T> ExtractDefusePayload<T> for SignedTip191Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload.0)
    }
}
