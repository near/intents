pub use near_global_contracts::StateInit;
use near_token::NearToken;

/// `DeterministicStateInit` action as per NEP-616
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
pub struct DeterministicStateInit {
    pub state_init: StateInit,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,
}

impl DeterministicStateInit {
    #[inline]
    pub fn new(state_init: impl Into<StateInit>) -> Self {
        Self {
            state_init: state_init.into(),
            deposit: NearToken::ZERO,
        }
    }

    #[inline]
    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }
}
