use near_sdk::{
    AccountId, Gas, GasWeight, NearToken, Promise,
    borsh::{self, BorshSerialize, io},
    near,
    serde::Serialize,
    serde_json,
    serde_with::base64::Base64,
    state_init::StateInit,
};

use crate::utils::is_default;

// TODO: remove
// p1.then(p2).and(p3).then_concurrent([p4, p5]).join().then(p6)

// TODO: deserialize as either [promises..], or {"after": [...], "promises": [...]}
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromiseDAG {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Self>,
    pub promises: Vec<PromiseSingle>,
}

impl PromiseDAG {
    pub fn new(promise: PromiseSingle) -> Self {
        Self {
            after: Vec::new(),
            promises: vec![promise],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.after.is_empty() && self.promises.is_empty()
    }

    // TODO:
    pub fn and(mut self, other: PromiseSingle) -> Self {
        self.promises.push(other);
        self
    }

    pub fn then(self, then: PromiseSingle) -> Self {
        Self {
            after: vec![self],
            promises: vec![then],
        }
    }

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

        Some(after.then_concurrent(promises).join())
    }
}

impl From<PromiseSingle> for PromiseDAG {
    fn from(promise: PromiseSingle) -> Self {
        Self::new(promise)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseSingle {
    // TODO: check that not self, otherise callbacks would be allowed to be
    // executed
    pub receiver_id: AccountId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_to: Option<AccountId>,

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

    pub fn build(self) -> Option<Promise> {
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
/// nor allow creating subaccounts.
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

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferAction {
    pub amount: NearToken,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInitAction {
    #[serde(flatten)]
    pub state_init: StateInit,
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub amount: NearToken,
}

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
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub amount: NearToken,
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

// TODO: remove
#[cfg(test)]
mod tests {
    use near_sdk::serde_json;

    use super::*;

    #[test]
    fn print_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&PromiseDAG {
                after: vec![PromiseDAG {
                    after: vec![
                        PromiseDAG {
                            after: vec![PromiseDAG {
                                after: vec![],
                                promises: vec![PromiseSingle {
                                    receiver_id: "p1".parse().unwrap(),
                                    refund_to: None,
                                    actions: vec![],
                                }],
                            }],
                            promises: vec![PromiseSingle {
                                receiver_id: "p2".parse().unwrap(),
                                refund_to: None,
                                actions: vec![],
                            }],
                        },
                        PromiseDAG {
                            after: vec![],
                            promises: vec![PromiseSingle {
                                receiver_id: "p3".parse().unwrap(),
                                refund_to: None,
                                actions: vec![],
                            }]
                        }
                    ],
                    promises: vec![
                        PromiseSingle {
                            receiver_id: "p4".parse().unwrap(),
                            refund_to: None,
                            actions: vec![],
                        },
                        PromiseSingle {
                            receiver_id: "p5".parse().unwrap(),
                            refund_to: None,
                            actions: vec![],
                        }
                    ]
                }],
                promises: vec![PromiseSingle {
                    receiver_id: "p6".parse().unwrap(),
                    refund_to: None,
                    actions: vec![],
                }]
            })
            .unwrap()
        );
    }
}
