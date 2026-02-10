use near_sdk::{
    AccountId, Gas, GasWeight, NearToken, Promise,
    borsh::{self, BorshSerialize, io},
    env, near, require,
    serde::Serialize,
    serde_json,
    serde_with::base64::Base64,
    state_init::StateInit,
};

use crate::utils::is_default;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromiseDAG {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Self>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promises: Vec<PromiseSingle>,
}

impl PromiseDAG {
    pub fn new(promise: PromiseSingle) -> Self {
        Self {
            after: Vec::new(),
            promises: vec![promise],
        }
    }

    pub fn and(mut self, other: impl Into<Self>) -> Self {
        let other = other.into();
        if self.after.is_empty() && other.after.is_empty() {
            self.promises.extend(other.promises);
            return self;
        }

        Self {
            after: vec![self, other],
            promises: vec![],
        }
    }

    pub fn then(self, then: PromiseSingle) -> Self {
        self.then_concurrent([then])
    }

    pub fn then_concurrent(mut self, then: impl IntoIterator<Item = PromiseSingle>) -> Self {
        if self.promises.is_empty() {
            self.promises.extend(then);
            return self;
        }

        Self {
            after: vec![self],
            promises: then.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.after.is_empty() && self.promises.is_empty()
    }

    /// Returns the length of the longest chain of subsequent action
    /// receipts to be created.
    pub fn depth(&self) -> usize {
        self.after
            .iter()
            .map(Self::depth)
            .max()
            .unwrap_or(0)
            .saturating_add(self.promises.len().min(1))
    }

    /// Returns the total number of action receipts to be created.
    pub fn total_count(&self) -> usize {
        self.after
            .iter()
            .map(Self::total_count)
            .sum::<usize>()
            .saturating_add(self.promises.len())
    }

    pub fn normalize(&mut self) {
        self.after.retain_mut(|after| {
            after.normalize();
            !after.is_empty()
        });
        self.promises.retain(|p| !p.is_empty());
    }

    // TODO: check that not self, otherise callbacks would be allowed to be
    // executed
    pub fn build(self) -> Option<Promise> {
        let promises = self.promises.into_iter().filter_map(PromiseSingle::build);

        let Some(after) = self
            .after
            .into_iter()
            .filter_map(Self::build)
            .reduce(Promise::and)
        else {
            return promises.reduce(Promise::and);
        };

        let mut promises = promises.peekable();
        if promises.peek().is_none() {
            return Some(after);
        }

        // `.then_concurrent([single])` is equivalent to `.then(single)`
        Some(after.then_concurrent(promises).join())
    }
}

impl From<PromiseSingle> for PromiseDAG {
    fn from(promise: PromiseSingle) -> Self {
        Self::new(promise)
    }
}

impl IntoIterator for PromiseDAG {
    type Item = PromiseSingle;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(vec![self])
    }
}

#[derive(Debug, Clone)]
pub struct IntoIter(Vec<PromiseDAG>);

impl Iterator for IntoIter {
    type Item = PromiseSingle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.0.last_mut()?.promises.pop() {
                return Some(p);
            }
            let d = self.0.pop()?;
            self.0.extend(d.after);
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.last().map(|d| d.promises.len()).unwrap_or(0), None)
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
/// A single outgoing receipt
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseSingle {
    pub receiver_id: AccountId,

    /// Receiver for refunds of failed or unused deposits.
    /// By default, it's the wallet-contract itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_to: Option<AccountId>,

    /// Empty actions is no-op.
    pub actions: Vec<PromiseAction>,
}

impl PromiseSingle {
    pub fn new(receiver_id: impl Into<AccountId>) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            refund_to: None,
            actions: Vec::new(),
        }
    }

    pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
        self.refund_to = Some(account_id.into());
        self
    }

    pub fn transfer(self, amount: NearToken) -> Self {
        self.add_action(PromiseAction::Transfer(TransferAction { amount }))
    }

    pub fn state_init(self, state_init: StateInit, amount: NearToken) -> Self {
        self.add_action(PromiseAction::StateInit(StateInitAction {
            state_init,
            amount,
        }))
    }

    pub fn function_call(self, action: FunctionCallAction) -> Self {
        self.add_action(PromiseAction::FunctionCall(action))
    }

    fn add_action(mut self, action: PromiseAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn and(self, other: impl Into<PromiseDAG>) -> PromiseDAG {
        PromiseDAG::from(self).and(other)
    }

    pub fn then(self, then: Self) -> PromiseDAG {
        PromiseDAG::from(self).then(then)
    }

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

/// NOTE: there is no support for other actions, since they operate on the
/// account itself (e.g. DeployContract, AddKey and etc...) or its on children
/// (e.g. CreateAccount). Wallet-contracts are not self-upgradable and do
/// not allow creating subaccounts.
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[serde(tag = "action", content = "args", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromiseAction {
    Transfer(TransferAction),
    StateInit(StateInitAction),
    FunctionCall(FunctionCallAction),
}

impl PromiseAction {
    pub fn append(self, p: Promise) -> Promise {
        match self {
            Self::Transfer(a) => p.transfer(a.amount),
            Self::StateInit(a) => p.state_init(a.state_init, a.amount),
            Self::FunctionCall(a) => p.function_call_weight(
                a.function_name,
                a.args,
                a.amount,
                a.min_gas,
                GasWeight(a.gas_weight),
            ),
        }
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferAction {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::utils::arbitrary::near_token),
    )]
    pub amount: NearToken,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInitAction {
    #[serde(flatten)]
    pub state_init: StateInit,
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::utils::arbitrary::near_token),
    )]
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub amount: NearToken,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCallAction {
    pub function_name: String,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    #[serde_as(as = "Base64")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<u8>,

    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::utils::arbitrary::near_token),
    )]
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub amount: NearToken,

    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::utils::arbitrary::gas),
    )]
    #[serde(default, skip_serializing_if = "Gas::is_zero")]
    pub min_gas: Gas,

    #[serde(default, skip_serializing_if = "is_default")]
    pub gas_weight: u64,
}

impl FunctionCallAction {
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            args: vec![],
            amount: NearToken::ZERO,
            min_gas: Gas::from_gas(0),
            gas_weight: 1,
        }
    }

    pub fn args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    pub fn args_json<T>(self, args: T) -> serde_json::Result<Self>
    where
        T: Serialize,
    {
        serde_json::to_vec(&args).map(|args| self.args(args))
    }

    pub fn args_borsh<T>(self, args: T) -> io::Result<Self>
    where
        T: BorshSerialize,
    {
        borsh::to_vec(&args).map(|args| self.args(args))
    }

    pub fn attached_deposit(mut self, amount: NearToken) -> Self {
        self.amount = amount;
        self
    }

    pub fn min_gas(mut self, min_gas: Gas) -> Self {
        self.min_gas = min_gas;
        self
    }

    pub fn unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = gas_weight;
        self
    }

    pub fn exact_gas(self, gas: Gas) -> Self {
        self.min_gas(gas).unused_gas_weight(0)
    }
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;

#[cfg(test)]
mod tests {
    use defuse_tests::utils::random::make_arbitrary;
    use near_sdk::{env, serde_json};
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(PromiseDAG::default(), 0)]
    #[case(p(1), 1)]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)), 4)]
    fn test_depth(#[case] p: impl Into<PromiseDAG>, #[case] depth: usize) {
        assert_eq!(p.into().depth(), depth);
    }

    #[rstest]
    #[case(PromiseDAG::default(), 0)]
    #[case(p(1), 1)]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)), 6)]
    fn test_total_count(#[case] p: impl Into<PromiseDAG>, #[case] total_count: usize) {
        assert_eq!(p.into().total_count(), total_count);
    }

    #[rstest]
    #[case(PromiseDAG::default(), vec![])]
    #[case(p(1), vec![p(1)])]
    #[case(
        p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)),
        vec![p(1), p(2), p(3), p(4), p(5), p(6)],
    )]
    fn test_iter(#[case] d: impl Into<PromiseDAG>, #[case] mut expected: Vec<PromiseSingle>) {
        let mut ps = d.into().into_iter().collect::<Vec<_>>();

        // sort by hashes
        ps.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));
        expected.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));

        assert_eq!(ps, expected);
    }

    #[rstest]
    fn test_normalize(#[from(make_arbitrary)] mut d: PromiseDAG) {
        d.normalize();
        check_json(d);
    }

    #[rstest]
    #[case(PromiseDAG::default())]
    #[case(p(1))]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)))]
    fn check_json(#[case] d: impl Into<PromiseDAG>) {
        println!("{}", serde_json::to_string_pretty(&d.into()).unwrap());
    }

    #[rstest]
    fn arbitrary_json(#[from(make_arbitrary)] d: PromiseDAG) {
        check_json(d);
    }

    fn p(n: usize) -> PromiseSingle {
        PromiseSingle::new(format!("p{n}").parse::<AccountId>().unwrap())
    }
}
