pub mod erc191;
pub mod multi;
pub mod nep413;
pub mod raw;
pub mod sep53;
pub mod tip191;
pub mod ton_connect;
pub mod webauthn;

use core::convert::Infallible;

use impl_tools::autoimpl;
use near_sdk::{AccountId, CryptoHash};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::{Nonce, Timestamp};

// TODO: add version
#[serde_as]
#[autoimpl(Deref using self.message)]
#[autoimpl(DerefMut using self.message)]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefusePayload<T> {
    pub signer_id: AccountId,
    pub verifying_contract: AccountId,
    pub deadline: Timestamp,
    #[serde_as(as = "Base64")]
    #[cfg_attr(feature = "abi", schemars(example = "self::examples::nonce"))]
    pub nonce: Nonce,

    #[serde(flatten)]
    pub message: T,
}

pub trait ExtractDefusePayload<T> {
    type Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error>;
}

impl<T> ExtractDefusePayload<T> for DefusePayload<T> {
    type Error = Infallible;

    #[inline]
    fn extract_defuse_payload(self) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

/// Data that can be deterministically hashed for signing or verification.
///
/// Implementations of this trait typically represent a message formatted
/// according to an external signing standard. The [`.hash()`](Self::hash)
/// method returns the digest that should be signed or used for verification.
pub trait Payload {
    fn hash(&self) -> CryptoHash;
}

/// Extension of [`Payload`] for types that include a signature.
///
/// Implementers verify the signature and, when successful, return the
/// signer's public key. This trait is mainly intended for internal use and
/// does not constitute a stable public API.
pub trait SignedPayload: Payload {
    type PublicKey;

    fn verify(&self) -> Option<Self::PublicKey>;
}

#[cfg(feature = "abi")]
mod examples {
    use super::Nonce;

    use near_sdk::base64::{self, Engine};

    pub fn nonce() -> String {
        base64::engine::general_purpose::STANDARD.encode(Nonce::default())
    }
}
