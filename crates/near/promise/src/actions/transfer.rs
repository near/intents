use near_token::NearToken;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferAction {
    pub amount: NearToken,
}

impl From<NearToken> for TransferAction {
    #[inline]
    fn from(amount: NearToken) -> Self {
        Self { amount }
    }
}
