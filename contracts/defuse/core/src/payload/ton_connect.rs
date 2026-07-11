use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use defuse_ton_connect::TonConnect;
pub use defuse_ton_connect::{TonConnectPayload, TonConnectPayloadSchema};
use near_sdk::CryptoHash;
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, Error},
};

use crate::payload::{Payload, SignedPayload};

use super::{DefusePayload, ExtractDefusePayload};

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTonConnectPayload {
    #[serde(flatten)]
    pub payload: TonConnectPayload,

    pub public_key: Ed25519PublicKey,
    pub signature: Ed25519Signature,
}

impl Payload for SignedTonConnectPayload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.try_prehash().expect("ton-connect hash")
    }
}

impl SignedPayload for SignedTonConnectPayload {
    type PublicKey = Ed25519PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        TonConnect::verify(
            &self.public_key.try_into().ok()?,
            &self.payload,
            &self.signature.into(),
        )
        .then_some(&self.public_key)
        .copied()
    }
}

impl<T> ExtractDefusePayload<T> for SignedTonConnectPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<T> ExtractDefusePayload<T> for TonConnectPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        let TonConnectPayloadSchema::Text { text } = self.payload else {
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
        if p.deadline < self.timestamp {
            return Err(Error::custom("deadline < timestamp"));
        }

        Ok(p)
    }
}
