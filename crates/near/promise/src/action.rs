use derive_more::From;
use near_gas::NearGas;
use near_token::NearToken;

// TODO: migrate to near-global-contracts
use near_sdk::state_init::StateInit;

/// NOTE: there is no support for other actions, since they operate on the
/// account itself (e.g. DeployContract, AddKey and etc...) or on its subaccounts
/// (e.g. CreateAccount). Wallet-contracts are not self-upgradable and do
/// not allow creating subaccounts.
#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema)),
    serde(tag = "action", rename_all = "snake_case")
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
#[repr(u8)] // matches nearcore `Action` just in case
pub enum PromiseAction {
    FunctionCall(FunctionCallAction) = 2,
    Transfer(TransferAction) = 3,
    StateInit(StateInitAction) = 11,
}

impl PromiseAction {
    pub const fn deposit(&self) -> NearToken {
        match self {
            Self::FunctionCall(FunctionCallAction { deposit, .. })
            | Self::Transfer(TransferAction { amount: deposit })
            | Self::StateInit(StateInitAction { deposit, .. }) => *deposit,
        }
    }

    pub const fn estimate_gas(&self) -> NearGas {
        match self {
            Self::FunctionCall(FunctionCallAction { min_gas, .. }) => *min_gas,
            // estimated for Near Implicit AccountId of receiver
            // (most expensive one)
            Self::Transfer(_) => NearGas::from_tgas(12),
            // estimated for state_init that fits in ZBA limits
            Self::StateInit(_) => NearGas::from_tgas(15),
        }
    }
}

#[cfg(feature = "near-contract")]
const _: () = {
    use near_sdk::{GasWeight, Promise};

    impl PromiseAction {
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
};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferAction {
    pub amount: NearToken,
}

/// `DeterministicStateInit` action as per NEP-616
#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateInitAction {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub state_init: StateInit,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionCallAction {
    pub function_name: String,

    #[cfg_attr(
        feature = "serde",
        cfg_attr(feature = "abi", schemars(with = "String")),
        serde_as(as = "::serde_with::base64::Base64"),
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub args: Vec<u8>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearGas::is_zero")
    )]
    pub min_gas: NearGas,

    #[cfg_attr(
        feature = "serde",
        serde_as(as = "::serde_with::DisplayFromStr"),
        serde(
            default = "default_gas_weight",
            skip_serializing_if = "is_default_gas_weight"
        )
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
            min_gas: NearGas::from_gas(0),
            gas_weight: default_gas_weight(),
        }
    }

    #[must_use]
    pub fn args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    #[cfg(feature = "json")]
    #[must_use]
    pub fn args_json<T>(self, args: T) -> Self
    where
        T: ::serde::Serialize,
    {
        self.args(::serde_json::to_vec(&args).unwrap())
    }

    #[cfg(feature = "borsh")]
    #[must_use]
    pub fn args_borsh<T>(self, args: T) -> Self
    where
        T: ::borsh::BorshSerialize,
    {
        self.args(borsh::to_vec(&args).unwrap())
    }

    #[must_use]
    pub const fn attached_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    #[must_use]
    pub const fn min_gas(mut self, min_gas: NearGas) -> Self {
        self.min_gas = min_gas;
        self
    }

    #[must_use]
    pub const fn unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = gas_weight;
        self
    }

    #[must_use]
    pub const fn exact_gas(self, gas: NearGas) -> Self {
        self.min_gas(gas).unused_gas_weight(0)
    }
}

fn default_gas_weight() -> u64 {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_gas_weight(gas_weight: &u64) -> bool {
    *gas_weight == default_gas_weight()
}
