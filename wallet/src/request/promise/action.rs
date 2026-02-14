use near_sdk::{
    Gas, GasWeight, NearToken, Promise,
    borsh::{self, BorshSerialize},
    near,
    serde::Serialize,
    serde_json,
    serde_with::{DisplayFromStr, base64::Base64},
    state_init::StateInit,
};

/// NOTE: there is no support for other actions, since they operate on the
/// account itself (e.g. DeployContract, AddKey and etc...) or its on children
/// (e.g. CreateAccount). Wallet-contracts are not self-upgradable and do
/// not allow creating subaccounts.
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "action", content = "args", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)] // matches nearcore `Action` just in case
pub enum PromiseAction {
    FunctionCall(FunctionCallAction) = 2,
    Transfer(TransferAction) = 3,
    StateInit(StateInitAction) = 11,
}

impl PromiseAction {
    pub const fn deposit(&self) -> NearToken {
        #[allow(clippy::match_same_arms)]
        match self {
            Self::FunctionCall(FunctionCallAction { deposit, .. }) => *deposit,
            Self::Transfer(TransferAction { amount }) => *amount,
            Self::StateInit(StateInitAction { deposit, .. }) => *deposit,
        }
    }

    pub const fn estimate_gas(&self) -> Gas {
        match self {
            Self::FunctionCall(FunctionCallAction { min_gas, .. }) => *min_gas,
            // estimated for Near Implicit AccountId of receiver
            // (most expensive one)
            Self::Transfer(_) => Gas::from_tgas(12),
            // estimated for state_init that fits in ZBA limits
            Self::StateInit(_) => Gas::from_tgas(15),
        }
    }

    pub(crate) fn append(self, p: Promise) -> Promise {
        match self {
            Self::Transfer(a) => p.transfer(a.amount),
            Self::StateInit(a) => p.state_init(a.state_init, a.deposit),
            Self::FunctionCall(a) => p.function_call_weight(
                a.function_name,
                a.args,
                a.deposit,
                a.min_gas,
                GasWeight(a.gas_weight),
            ),
        }
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferAction {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::arbitrary::near_token),
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
        arbitrary(with = crate::arbitrary::near_token),
    )]
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub deposit: NearToken,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCallAction {
    pub function_name: String,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    #[serde_as(as = "Base64")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<u8>,

    // TODO: rename: deposit
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::arbitrary::near_token),
    )]
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub deposit: NearToken,

    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = crate::arbitrary::gas),
    )]
    #[serde(default, skip_serializing_if = "Gas::is_zero")]
    pub min_gas: Gas,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(
        default = "default_gas_weight",
        skip_serializing_if = "is_default_gas_weight"
    )]
    pub gas_weight: u64,
}

impl FunctionCallAction {
    #[must_use]
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            args: vec![],
            deposit: NearToken::ZERO,
            min_gas: Gas::from_gas(0),
            gas_weight: 1,
        }
    }

    #[must_use]
    pub fn args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    #[must_use]
    pub fn args_json<T>(self, args: T) -> Self
    where
        T: Serialize,
    {
        self.args(serde_json::to_vec(&args).unwrap())
    }

    #[must_use]
    pub fn args_borsh<T>(self, args: T) -> Self
    where
        T: BorshSerialize,
    {
        self.args(borsh::to_vec(&args).unwrap())
    }

    #[must_use]
    pub const fn attached_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    #[must_use]
    pub const fn min_gas(mut self, min_gas: Gas) -> Self {
        self.min_gas = min_gas;
        self
    }

    #[must_use]
    pub const fn unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = gas_weight;
        self
    }

    #[must_use]
    pub const fn exact_gas(self, gas: Gas) -> Self {
        self.min_gas(gas).unused_gas_weight(0)
    }
}

fn default_gas_weight() -> u64 {
    GasWeight::default().0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_gas_weight(gas_weight: &u64) -> bool {
    *gas_weight == default_gas_weight()
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
