use defuse_crypto::{Curve, CurveTypes, Ed25519};
use defuse_ton_connect::{SignedTonConnectPayload, TonConnectPayload, TonConnectPayloadSchema};
use digest::{Digest, consts::U32};
use near_sdk::{
    serde::de::{DeserializeOwned, Error},
    serde_json,
};

use super::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};

impl<D: Digest<OutputSize = U32>> Payload for TonConnectPayload<D> {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        self.hash()
    }
}

impl<D: Digest<OutputSize = U32>> Payload for SignedTonConnectPayload<D> {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        Payload::hash(&self.payload)
    }
}

impl<D: Digest<OutputSize = U32>> SignedPayload for SignedTonConnectPayload<D> {
    type PublicKey = <Ed25519 as CurveTypes>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &Payload::hash(self), &self.public_key)
    }
}

impl<D: Digest<OutputSize = U32>, T> ExtractDefusePayload<T> for SignedTonConnectPayload<D>
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<D: Digest<OutputSize = U32>, T> ExtractDefusePayload<T> for TonConnectPayload<D>
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        #[allow(irrefutable_let_patterns)]
        let TonConnectPayloadSchema::Text(text) = self.payload else {
            return Err(Error::custom("only text payload supported"));
        };

        let p: DefusePayload<T> = serde_json::from_str(&text)?;

        // TON Connect [specification](https://docs.tonconsole.com/academy/sign-data#in-a-smart-contract-on-chain)
        // requires to check that "timestamp is recent". We don't have fixed TTL
        // for off-chain signatures but rather check if `deadline` is not expired.
        //
        // At first, we were asserting `(timestamp <= now())`, but that  was causing
        // `simulate_intents()` to fail, since sometimes signed intent is simulated
        // right after signing.
        //
        // So, we ended up to assert at least following:
        if p.deadline.into_timestamp() < self.timestamp {
            return Err(Error::custom("deadline < timestamp"));
        }

        Ok(p)
    }
}
