use near_sdk::{AccountId, Gas, NearToken, Promise, env, near, require, state_init::StateInit};

use crate::{FunctionCallAction, PromiseAction, PromiseDAG, StateInitAction, TransferAction};

/// A single outgoing receipt
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseSingle {
    /// Receiver of the receipt to be created.
    ///
    /// NOTE: self-calls are prohibited.
    pub receiver_id: AccountId,

    /// Receiver for refunds of failed or unused deposits.
    /// By default, it's the wallet-contract itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_to: Option<AccountId>,

    /// Empty actions is no-op.
    pub actions: Vec<PromiseAction>,
}

impl PromiseSingle {
    #[must_use]
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
    pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
        self.refund_to = Some(account_id.into());
        self
    }

    #[must_use]
    pub fn transfer(self, amount: NearToken) -> Self {
        self.add_action(PromiseAction::Transfer(TransferAction { amount }))
    }

    #[must_use]
    pub fn state_init(self, state_init: StateInit, deposit: NearToken) -> Self {
        self.add_action(PromiseAction::StateInit(StateInitAction {
            state_init,
            deposit,
        }))
    }

    #[must_use]
    pub fn function_call(self, action: FunctionCallAction) -> Self {
        self.add_action(PromiseAction::FunctionCall(action))
    }

    fn add_action(mut self, action: PromiseAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Returns whether the promise is no-op, i.e. list of actions is empty
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns total NEAR deposit for all actions in this promise
    pub fn total_deposit(&self) -> NearToken {
        self.actions
            .iter()
            .map(PromiseAction::deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    /// Returns an esitmate of mininum gas required to execute all
    /// actions in this promise
    pub fn estimate_gas(&self) -> Gas {
        self.actions
            .iter()
            .map(PromiseAction::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
    }

    /// Schedule another promise(s) to be executed concurrently to this one
    #[must_use]
    pub fn and(self, other: impl Into<PromiseDAG>) -> PromiseDAG {
        PromiseDAG::from(self).and(other)
    }

    /// Schedule another promise to be executed right after this one
    #[must_use]
    pub fn then(self, then: Self) -> PromiseDAG {
        PromiseDAG::from(self).then(then)
    }

    /// Schedule given promises to be executed concurrently right after this one
    #[must_use]
    pub fn then_concurrent(self, then: impl IntoIterator<Item = Self>) -> PromiseDAG {
        PromiseDAG::from(self).then_concurrent(then)
    }

    /// Build promise for execution
    #[must_use]
    pub fn build(self) -> Option<Promise> {
        // assert here instead of returning an error to reduce complexity
        require!(
            self.receiver_id != env::current_account_id(),
            "self-calls are prohibited",
        );

        if self.actions.is_empty() {
            return None;
        }

        let mut p = Promise::new(self.receiver_id);

        if let Some(refund_to) = self.refund_to {
            p = p.refund_to(refund_to);
        }

        Some(
            self.actions
                .into_iter()
                .fold(p, |p, action| action.append(p)),
        )
    }
}

#[cfg(test)]
mod tests {
    use arbitrary::{Arbitrary, Unstructured};
    use rstest::rstest;

    use crate::request::promise::tests::p;

    use super::*;

    #[rstest]
    #[case(p(0), Gas::from_gas(0))]
    #[case(
        p(0).function_call(
                FunctionCallAction::new("foo")
                .min_gas(Gas::from_tgas(123))
        ).function_call(
                FunctionCallAction::new("fbaro")
                .min_gas(Gas::from_tgas(45))
        ), Gas::from_tgas(123 + 45)
    )]
    fn estimate_gas(#[case] p: PromiseSingle, #[case] expected: Gas) {
        assert_eq!(p.estimate_gas(), expected);
    }

    #[rstest]
    #[case(p(0), NearToken::ZERO)]
    #[case(
        p(0)
            .transfer(NearToken::from_yoctonear(1))
            .state_init(
                Arbitrary::arbitrary(&mut Unstructured::new(&[])).unwrap(),
                NearToken::from_yoctonear(2)
            ).function_call(
                FunctionCallAction::new("foo")
                .attached_deposit(NearToken::from_yoctonear(3))
            ),
        NearToken::from_yoctonear(6),
    )]
    fn total_deposit(#[case] p: PromiseSingle, #[case] expected: NearToken) {
        assert_eq!(p.total_deposit(), expected);
    }
}
