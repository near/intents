#[cfg(feature = "arbitrary")]
mod arbitrary;

use near_sdk::{
    Gas, GasWeight, NearToken, Promise,
    borsh::{self, BorshSerialize},
    near,
    serde::Serialize,
    serde_json,
    serde_with::{DisplayFromStr, base64::Base64},
};

pub trait AppendAction {
    fn append(self, p: Promise) -> Promise;
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferAction {
    #[cfg_attr(
        feature = "arbitrary", 
        arbitrary(with = crate::arbitrary::near_token),
    )]
    pub amount: NearToken,
}

impl AppendAction for TransferAction {
    fn append(self, p: Promise) -> Promise {
        p.transfer(self.amount)
    }
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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

    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = crate::arbitrary::near_token),
    )]
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub deposit: NearToken,

    #[cfg_attr(
        feature = "arbitrary", 
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
            gas_weight: default_gas_weight(),
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

impl AppendAction for FunctionCallAction {
    fn append(self, p: Promise) -> Promise {
        p.function_call_weight(
            self.function_name,
            self.args,
            self.deposit,
            self.min_gas,
            GasWeight(self.gas_weight),
        )
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
