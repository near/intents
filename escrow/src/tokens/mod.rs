#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use near_sdk::{Gas, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::token_id::{TokenId, TokenIdType};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Sent {
    pub token_type: TokenIdType,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub is_call: bool,
}

pub trait TokenIdExt: Sized {
    fn token_type(&self) -> TokenIdType;

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas);

    fn transfer_gas(&self, min_gas: Option<Gas>, is_call: bool) -> Gas {
        let (min, default) = self.transfer_gas_min_default(is_call);
        min_gas.unwrap_or(default).max(min)
    }
}

impl TokenIdExt for TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        self.into()
    }

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141(token) => token.transfer_gas_min_default(is_call),
            #[cfg(feature = "nep245")]
            Self::Nep245(token) => token.transfer_gas_min_default(is_call),
        }
    }
}
