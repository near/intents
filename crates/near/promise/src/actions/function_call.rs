use std::borrow::Cow;

use near_gas::NearGas as Gas;
use near_token::NearToken;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionCallAction {
    pub function_name: String,

    #[cfg_attr(
        feature = "serde",
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
        // TODO: zero gas?
        serde(default, skip_serializing_if = "Gas::is_zero")
    )]
    pub gas: Gas,

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

// TODO: must use
impl FunctionCallAction {
    #[must_use]
    #[inline]
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            args: Vec::new(),
            deposit: NearToken::ZERO,
            gas: Gas::from_gas(30),
            gas_weight: default_gas_weight(),
        }
    }

    #[must_use]
    #[inline]
    pub fn args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    #[cfg(feature = "json")]
    #[inline]
    pub fn args_json<T>(self, args: &T) -> ::serde_json::Result<Self>
    where
        T: ::serde::Serialize,
    {
        Ok(self.args(::serde_json::to_vec(args)?))
    }

    #[cfg(feature = "borsh")]
    #[inline]
    pub fn args_borsh<T>(self, args: &T) -> ::borsh::io::Result<Self>
    where
        T: ::borsh::BorshSerialize,
    {
        Ok(self.args(::borsh::to_vec(args)?))
    }

    #[must_use]
    #[inline]
    pub const fn attached_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    #[must_use]
    #[inline]
    pub const fn gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    #[must_use]
    #[inline]
    pub const fn unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = gas_weight;
        self
    }

    #[must_use]
    #[inline]
    pub const fn gas_exact(self, gas: Gas) -> Self {
        self.gas(gas).unused_gas_weight(0)
    }
}

impl From<String> for FunctionCallAction {
    #[inline]
    fn from(function_name: String) -> Self {
        Self::new(function_name)
    }
}

impl From<&String> for FunctionCallAction {
    #[inline]
    fn from(function_name: &String) -> Self {
        Self::new(function_name)
    }
}

impl From<&str> for FunctionCallAction {
    #[inline]
    fn from(function_name: &str) -> Self {
        Self::new(function_name)
    }
}

impl From<Cow<'_, str>> for FunctionCallAction {
    #[inline]
    fn from(function_name: Cow<str>) -> Self {
        Self::new(function_name)
    }
}

const fn default_gas_weight() -> u64 {
    1
}

#[cfg(feature = "serde")]
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_default_gas_weight(gas_weight: &u64) -> bool {
    *gas_weight == default_gas_weight()
}
