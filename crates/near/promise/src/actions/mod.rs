mod function_call;
mod state_init;
mod transfer;

pub use self::{function_call::*, state_init::*, transfer::*};

pub use near_gas::NearGas as Gas;
pub use near_token::NearToken;

use derive_more::From;

/// NOTE: there is no support for other actions, since they operate on the
/// account itself (e.g. `DeployContract`, `AddKey` and etc...) or on its subaccounts
/// (e.g. `CreateAccount`). Wallet-contracts are not self-upgradable and do
/// not allow creating subaccounts.
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
#[repr(u8)] // matches nearcore `Action` just in case
pub enum NearAction {
    FunctionCall(FunctionCallAction) = 2,
    Transfer(TransferAction) = 3,
    StateInit(StateInitAction) = 11,
}

impl NearAction {
    #[must_use]
    #[inline]
    pub const fn deposit(&self) -> NearToken {
        match self {
            Self::FunctionCall(FunctionCallAction { deposit, .. })
            | Self::Transfer(TransferAction { amount: deposit })
            | Self::StateInit(StateInitAction { deposit, .. }) => *deposit,
        }
    }

    #[must_use]
    #[inline]
    pub const fn estimate_gas(&self) -> Gas {
        match self {
            Self::FunctionCall(FunctionCallAction { gas, .. }) => *gas,
            // estimated for Near Implicit AccountId of receiver
            // (most expensive one)
            Self::Transfer(_) => Gas::from_tgas(12),
            // estimated for state_init that fits in ZBA limits
            Self::StateInit(_) => Gas::from_tgas(15),
        }
    }
}

#[cfg(feature = "near-contract")]
const _: () = {
    use near_sdk::{GasWeight, Promise};

    impl NearAction {
        pub(crate) fn append(self, p: Promise) -> Promise {
            match self {
                Self::Transfer(a) => p.transfer(a.amount),
                Self::StateInit(a) => {
                    // TODO: use `promise_batch_action_state_init_raw` when
                    // Universal Implicit AccountIds lands
                    use near_global_contracts::StateInit;

                    let state_init: StateInit = borsh::from_slice(&a.state_init)
                        .unwrap_or_else(|e| panic!("cannot borsh-deserialize StateInit: {e}"));

                    p.state_init(state_init, a.deposit)
                }
                Self::FunctionCall(a) => p.function_call_weight(
                    a.function_name,
                    a.args,
                    a.deposit,
                    a.gas,
                    GasWeight(a.gas_weight),
                ),
            }
        }
    }
};
