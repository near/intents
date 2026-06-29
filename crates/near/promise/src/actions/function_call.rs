use std::borrow::Cow;

use near_gas::NearGas as Gas;
use near_token::NearToken;

/// Function call action of [`NearPromise`](crate::NearPromise).
#[must_use = "promises do nothing unless you `.build()` them"]
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
pub struct FunctionCall {
    /// Name of the function to execute.
    pub function_name: String,

    /// Arguments passed to the function as an input.
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "::serde_with::base64::Base64"),
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub args: Vec<u8>,

    /// Deposit to attach to the function call
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,

    /// _Minimum_ amount of gas to be allocated for this function call.
    ///
    /// Unused gas will be distributed between all outgoing function calls
    /// created during caller's execution according to their
    /// [`gas_weight`](field@Self::gas_weight)s.
    ///
    /// Zero `gas` means that only unused gas will be distributed to this
    /// function call according to its [`gas_weight`](field@Self::gas_weight).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Gas::is_zero")
    )]
    pub gas: Gas,

    /// Weight for unused gas distribution.
    ///
    /// Zero `weight` means that no unused gas will be distributed to this
    /// function call, i.e. only exact [`gas`](field@Self::gas) will be
    /// allocated.
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

impl FunctionCall {
    /// Call a function with given name.
    ///
    /// NOTE: By default, at least `50 Tgas` will be allocated to this
    /// function call. See [`.gas()`](method@Self::gas) and
    /// [`.unused_gas_weight()`](method@Self::unused_gas_weight) methods
    /// for more info.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{Gas, actions::FunctionCall};
    /// let f = FunctionCall::name("foo");
    ///
    /// assert_eq!(f.function_name, "foo");
    /// assert!(f.args.is_empty(), "empty args");
    /// assert!(f.deposit.is_zero(), "zero deposit");
    /// assert_eq!(f.gas, Gas::from_tgas(50));
    /// assert_eq!(f.gas_weight, 1);
    /// ```
    #[inline]
    pub fn name(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            args: Vec::new(),
            deposit: NearToken::ZERO,
            gas: Gas::from_tgas(50),
            gas_weight: default_gas_weight(),
        }
    }

    /// Pass given args as an input to the function call.
    #[inline]
    pub fn args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    #[cfg(feature = "json")]
    /// Pass given args serialized as JSON as an input to the function call.
    ///
    /// # Panics
    ///
    /// Panics if JSON serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use serde_json::json;
    /// # use defuse_near_promise::actions::FunctionCall;
    /// let f = FunctionCall::name("foo")
    ///     .args_json(json!({"key": "value"}));
    ///
    /// assert_eq!(f.args, br#"{"key":"value"}"#);
    /// ```
    #[track_caller]
    #[inline]
    pub fn args_json<T>(self, args: T) -> Self
    where
        T: ::serde::Serialize,
    {
        self.args(::serde_json::to_vec(&args).expect("JSON serialization failed"))
    }

    #[cfg(feature = "borsh")]
    /// Pass given args serialized as borsh as an input to the function call.
    ///
    /// # Panics
    ///
    /// Panics if borsh serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::actions::FunctionCall;
    /// let f = FunctionCall::name("foo")
    ///     .args_borsh(123u32);
    ///
    /// assert_eq!(f.args, [0x7b, 0x00, 0x00, 0x00]);
    /// ```
    #[track_caller]
    #[inline]
    pub fn args_borsh<T>(self, args: T) -> Self
    where
        T: ::borsh::BorshSerialize,
    {
        self.args(::borsh::to_vec(&args).expect("borsh serialization failed"))
    }

    /// Attach deposit to this function call
    #[inline]
    pub const fn attach_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    /// Set a _minimum_ amount of gas to be allocated for this function call.
    /// See [`gas`](field@Self::gas)
    #[inline]
    pub const fn gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    /// Set unused gas weight for this function call.
    ///
    /// By default, gas weight of `1` is used.
    #[inline]
    pub const fn unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = gas_weight;
        self
    }

    #[allow(clippy::doc_link_code)]
    /// Set an _exact_ amount of gas to be allocated for this function call.
    ///
    /// This is identical to
    /// [`.gas(gas)`](method@Self::gas)[`.unused_gas_weight(0)`](method@Self::unused_gas_weight).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{Gas, actions::FunctionCall};
    /// let f = FunctionCall::name("foo")
    ///     .gas_exact(Gas::from_tgas(15));
    ///
    /// assert_eq!(f.gas, Gas::from_tgas(15));
    /// assert_eq!(f.gas_weight, 0);
    /// ```
    #[inline]
    pub const fn gas_exact(self, gas: Gas) -> Self {
        self.gas(gas).unused_gas_weight(0)
    }
}

impl From<String> for FunctionCall {
    #[inline]
    fn from(function_name: String) -> Self {
        Self::name(function_name)
    }
}

impl From<&String> for FunctionCall {
    #[inline]
    fn from(function_name: &String) -> Self {
        Self::name(function_name)
    }
}

impl From<&str> for FunctionCall {
    #[inline]
    fn from(function_name: &str) -> Self {
        Self::name(function_name)
    }
}

impl From<Cow<'_, str>> for FunctionCall {
    #[inline]
    fn from(function_name: Cow<str>) -> Self {
        Self::name(function_name)
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
