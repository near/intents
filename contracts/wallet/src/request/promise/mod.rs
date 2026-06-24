mod action;
pub use self::action::*;

use near_sdk::{AccountId, Gas, NearToken, Promise, near, state_init::StateInit};

/// A single outgoing receipt
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PromiseSingle {
    /// Receiver of the receipt to be created.
    ///
    /// NOTE: self-calls are prohibited.
    pub receiver_id: AccountId,

    /// Receiver for refunds of failed or unused deposits.
    /// By default, it's the wallet-contract itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_to: Option<AccountId>,

    /// Actions to execute within a single promise.
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
        self.add_action(TransferAction { amount })
    }

    #[must_use]
    pub fn state_init(self, state_init: StateInit, deposit: NearToken) -> Self {
        self.add_action(StateInitAction {
            state_init,
            deposit,
        })
    }

    #[must_use]
    pub fn function_call(self, action: FunctionCallAction) -> Self {
        self.add_action(action)
    }

    fn add_action(mut self, action: impl Into<PromiseAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Returns whether the promise is no-op, i.e. list of actions is empty
    pub const fn is_empty(&self) -> bool {
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

#[cfg(test)]
mod tests {
    use arbitrary::{Arbitrary, Unstructured};
    use rstest::rstest;

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

    fn p(n: usize) -> PromiseSingle {
        PromiseSingle::new(format!("p{n}").parse::<AccountId>().unwrap())
    }
}
