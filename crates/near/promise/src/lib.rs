pub mod actions;

pub use near_account_id::{self as account_id, AccountId, AccountIdRef};
pub use near_gas::NearGas as Gas;
// TODO
pub use near_global_contracts::StateInit;
pub use near_token::NearToken;

use self::actions::{FunctionCallAction, NearAction, StateInitAction, TransferAction};

/// A single outgoing receipt
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NearPromise {
    /// Receiver of the receipt to be created.
    ///
    /// NOTE: self-calls are prohibited.
    pub receiver_id: AccountId,

    /// Receiver for refunds of failed or unused deposits.
    /// By default, it's the wallet-contract itself.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub refund_to: Option<AccountId>,

    /// Actions to execute within a single promise.
    ///
    /// TODO: what empty list will result in?
    pub actions: Vec<NearAction>,
}

impl NearPromise {
    #[must_use]
    #[inline]
    pub fn new(receiver_id: impl Into<AccountId>) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            refund_to: None,
            actions: Vec::new(),
        }
    }

    /// Set an account where all failed/unused deposits should be refunded
    /// instead of the wallet-contract itself.
    #[must_use]
    #[inline]
    pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
        self.refund_to = Some(account_id.into());
        self
    }

    #[must_use]
    #[inline]
    pub fn transfer(self, amount: NearToken) -> Self {
        self.add_action(TransferAction { amount })
    }

    // TODO
    #[must_use]
    #[inline]
    pub fn state_init(self, state_init: impl Into<Vec<u8>>, deposit: NearToken) -> Self {
        self.add_action(StateInitAction::new(state_init).deposit(deposit))
    }

    /// Add `FunctionCall` action to this promise
    ///
    /// ```rust
    /// # use serde_json::json;
    /// # use defuse_near_promise::{AccountIdRef, Gas, NearToken};
    /// use defuse_near_promise::{NearPromise, actions::FunctionCallAction};
    ///
    /// let contract_id = AccountIdRef::new_or_panic("ft.near");
    ///
    /// let _ = NearPromise::new(contract_id)
    ///     .function_call(
    ///         FunctionCallAction::new("ft_transfer_call")
    ///             .args_json(&json!({
    ///                 "receiver_id": "receiver.near",
    ///                 "amount": "1000",
    ///                 "msg": "message",
    ///             }))
    ///                 .expect("failed to serialize JSON")
    ///             .attached_deposit(NearToken::from_yoctonear(1))
    ///             .gas(Gas::from_tgas(100)),
    ///     );
    /// ```
    #[must_use]
    #[inline]
    pub fn function_call(self, action: impl Into<FunctionCallAction>) -> Self {
        self.add_action(action.into())
    }

    #[must_use]
    #[inline]
    fn add_action(mut self, action: impl Into<NearAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Returns whether the promise is no-op, i.e. list of actions is empty
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns total NEAR deposit for all actions in this promise
    #[must_use]
    #[inline]
    pub fn total_deposit(&self) -> NearToken {
        self.actions
            .iter()
            .map(NearAction::deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    /// Returns an esitmate of mininum gas required to execute all
    /// actions in this promise
    #[must_use]
    #[inline]
    pub fn estimate_gas(&self) -> Gas {
        self.actions
            .iter()
            .map(NearAction::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
    }
}

#[cfg(feature = "near-contract")]
const _: () = {
    use near_sdk::Promise;

    impl NearPromise {
        /// Build promise for execution
        pub fn build(self) -> Promise {
            let mut p = Promise::new(self.receiver_id);

            if let Some(refund_to) = self.refund_to {
                p = p.refund_to(refund_to);
            }

            self.actions
                .into_iter()
                .fold(p, |p, action| action.append(p))
        }
    }
};

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(p(0), Gas::from_gas(0))]
    #[case(
        p(0)
            .function_call(FunctionCallAction::new("foo").gas(Gas::from_tgas(123)))
            .function_call(FunctionCallAction::new("bar").gas(Gas::from_tgas(45))),
        Gas::from_tgas(123 + 45)
    )]
    fn estimate_gas(#[case] p: NearPromise, #[case] expected: Gas) {
        assert_eq!(p.estimate_gas(), expected);
    }

    #[rstest]
    #[case(p(0), NearToken::ZERO)]
    #[case(
        p(0)
            .transfer(NearToken::from_yoctonear(1))
            .state_init(
                [],
                NearToken::from_yoctonear(2)
            ).function_call(
                FunctionCallAction::new("foo")
                .attached_deposit(NearToken::from_yoctonear(3))
            ),
        NearToken::from_yoctonear(6),
    )]
    fn total_deposit(#[case] p: NearPromise, #[case] expected: NearToken) {
        assert_eq!(p.total_deposit(), expected);
    }

    fn p(n: usize) -> NearPromise {
        NearPromise::new(format!("p{n}").parse::<AccountId>().unwrap())
    }
}
