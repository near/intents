mod function_call;
mod state_init;
mod transfer;

pub use self::{function_call::*, state_init::*, transfer::*};

pub use near_gas::NearGas as Gas;
pub use near_token::NearToken;

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

    /// TODO
    StateInit(StateInitAction) = 11,
}

impl NearAction {
    /// Returns NEAR deposit for this action.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{
    /// #     NearToken,
    /// #     actions::{NearAction, Transfer, FunctionCall},
    /// # };
    /// let transfer = NearAction::Transfer(NearToken::from_near(1).into());
    /// assert_eq!(transfer.deposit(), NearToken::from_near(1));
    ///
    /// let function_call: NearAction = FunctionCall::name("foo")
    ///     .attach_deposit(NearToken::from_near(5))
    ///     .into();
    /// assert_eq!(function_call.deposit(), NearToken::from_near(5));
    /// ```
    #[inline]
    pub(crate) const fn deposit(&self) -> NearToken {
        match self {
            Self::FunctionCall(FunctionCall { deposit, .. })
            | Self::Transfer(Transfer { amount: deposit })
            | Self::StateInit(StateInitAction { deposit, .. }) => *deposit,
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
                // TODO: might be a separate action?
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
