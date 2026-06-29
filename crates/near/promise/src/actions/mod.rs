mod function_call;
mod state_init;
mod transfer;

pub use self::{function_call::*, state_init::*, transfer::*};

use near_gas::NearGas as Gas;
use near_token::NearToken;

use derive_more::From;

/// A single action of [`NearPromise`](crate::NearPromise).
#[must_use = "promises do nothing unless you `.build()` them"]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(tag = "action", content = "payload", rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
#[non_exhaustive]
#[repr(u8)] // matches nearcore `Action` just in case
pub enum NearAction {
    /// [`FunctionCall`]
    FunctionCall(FunctionCall) = 2,

    /// [`Transfer`]
    Transfer(Transfer) = 3,

    /// [`DeterministicStateInit`]
    DeterministicStateInit(DeterministicStateInit) = 11,
}

impl NearAction {
    #[inline]
    pub(crate) const fn deposit(&self) -> NearToken {
        match self {
            Self::FunctionCall(FunctionCall { deposit, .. })
            | Self::Transfer(Transfer { amount: deposit })
            | Self::DeterministicStateInit(DeterministicStateInit { deposit, .. }) => *deposit,
        }
    }

    #[inline]
    pub(crate) const fn estimate_gas(&self) -> Gas {
        match self {
            Self::FunctionCall(FunctionCall { gas, .. }) => *gas,
            // estimated for Near Implicit AccountId of receiver
            // (most expensive one)
            Self::Transfer(_) => Gas::from_tgas(12),
            // estimated for state_init that fits in ZBA limits
            Self::DeterministicStateInit(_) => Gas::from_tgas(15),
        }
    }
}

#[cfg(feature = "near-contract")]
const _: () = {
    use near_sdk::{GasWeight, Promise};

    impl NearAction {
        pub(crate) fn append(self, p: Promise) -> Promise {
            match self {
                Self::FunctionCall(a) => p.function_call_weight(
                    a.function_name,
                    a.args,
                    a.deposit,
                    a.gas,
                    GasWeight(a.gas_weight),
                ),
                Self::Transfer(a) => p.transfer(a.amount),
                Self::DeterministicStateInit(a) => p.state_init(a.state_init, a.deposit),
            }
        }
    }
};
