use near_token::NearToken;

/// Transfer [action](crate::actions::NearAction).
#[must_use = "promises do nothing unless you `.build()` them"]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Transfer {
    /// Amount of NEAR tokens to transfer
    pub amount: NearToken,
}

impl From<NearToken> for Transfer {
    #[inline]
    fn from(amount: NearToken) -> Self {
        Self { amount }
    }
}
