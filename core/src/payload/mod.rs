pub mod erc191;
pub mod multi;
pub mod nep413;
pub mod raw;
pub mod webauthn;

use core::convert::Infallible;

use defuse_serde_utils::base64::Base64;
use impl_tools::autoimpl;
use near_sdk::{AccountId, near};
use serde_with::serde_as;

use crate::{Deadline, Nonce};

// TODO: add version
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.message)]
#[autoimpl(DerefMut using self.message)]
#[derive(Debug, Clone)]
pub struct DefusePayload<T> {
    pub signer_id: AccountId,
    pub verifying_contract: AccountId,
    pub deadline: Deadline,
    #[serde_as(as = "Base64")]
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(example = "self::examples::nonce")
    )]
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
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        Ok(self)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod examples {
    use super::*;

    use near_sdk::base64::{self, Engine};

    pub fn nonce() -> String {
        base64::engine::general_purpose::STANDARD.encode(Nonce::default())
    }
}
