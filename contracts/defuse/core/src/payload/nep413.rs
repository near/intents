use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
pub use defuse_nep413::{Nep413, Nep413Payload};
use impl_tools::autoimpl;
use near_sdk::AccountId;
use serde::{
    Deserialize, Serialize,
    de::{self, DeserializeOwned},
};
use serde_with::serde_as;

use crate::{
    Timestamp,
    payload::{Payload, SignedPayload},
};

use super::{DefusePayload, ExtractDefusePayload};

#[autoimpl(Deref using self.message)]
#[autoimpl(DerefMut using self.message)]
#[serde_as]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nep413DefuseMessage<T> {
    pub signer_id: AccountId,

    pub deadline: Timestamp,

    #[serde(flatten)]
    pub message: T,
}

impl<T> ExtractDefusePayload<T> for Nep413Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        let Nep413DefuseMessage {
            signer_id,
            deadline,
            message,
        } = serde_json::from_str(&self.message)?;

        Ok(DefusePayload {
            signer_id,
            verifying_contract: self.recipient.parse().map_err(|_| {
                de::Error::invalid_value(de::Unexpected::Str(&self.recipient), &"AccountId")
            })?,
            deadline,
            nonce: self.nonce,
            message,
        })
    }
}

#[autoimpl(Deref using self.payload)]
#[serde_as]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedNep413Payload {
    pub payload: Nep413Payload,

    pub public_key: Ed25519PublicKey,
    pub signature: Ed25519Signature,
}

impl Payload for SignedNep413Payload {
    fn hash(&self) -> near_sdk::CryptoHash {
        Nep413::prehash(&self.payload)
    }
}

impl SignedPayload for SignedNep413Payload {
    type PublicKey = Ed25519PublicKey;

    fn verify(&self) -> Option<Self::PublicKey> {
        Nep413::verify(
            &self.public_key.try_into().ok()?,
            &self.payload,
            &self.signature.into(),
        )
        .then_some(&self.public_key)
        .copied()
    }
}

impl<T> ExtractDefusePayload<T> for SignedNep413Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}
