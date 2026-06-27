use std::borrow::Cow;

use near_global_contracts::StateInit;
use near_token::NearToken;

/// `DeterministicStateInit` action as per NEP-616
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
pub struct StateInitAction {
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
    pub state_init: Vec<u8>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,
}

impl StateInitAction {
    #[must_use]
    #[inline]
    pub fn new(state_init: impl Into<Vec<u8>>) -> Self {
        Self {
            state_init: state_init.into(),
            deposit: NearToken::ZERO,
        }
    }

    #[must_use]
    #[inline]
    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }
}

impl From<Vec<u8>> for StateInitAction {
    #[inline]
    fn from(state_init: Vec<u8>) -> Self {
        Self::new(state_init)
    }
}

impl From<&[u8]> for StateInitAction {
    #[inline]
    fn from(state_init: &[u8]) -> Self {
        Self::new(state_init)
    }
}

impl From<Cow<'_, [u8]>> for StateInitAction {
    #[inline]
    fn from(state_init: Cow<[u8]>) -> Self {
        Self::new(state_init)
    }
}

#[cfg(feature = "borsh")]
const _: () = {
    use near_global_contracts::StateInitV1;

    impl From<&StateInit> for StateInitAction {
        #[inline]
        fn from(value: &StateInit) -> Self {
            Self::new(::borsh::to_vec(value).unwrap_or_else(|_| unreachable!()))
        }
    }

    impl From<StateInit> for StateInitAction {
        #[inline]
        fn from(value: StateInit) -> Self {
            Self::from(&value)
        }
    }

    impl From<StateInitV1> for StateInitAction {
        #[inline]
        fn from(value: StateInitV1) -> Self {
            StateInit::V1(value).into()
        }
    }
};
