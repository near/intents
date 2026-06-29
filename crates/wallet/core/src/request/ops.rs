use near_account_id::AccountId;

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    // TODO: tag/content
    serde(tag = "op", content = "payload", rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum WalletOp {
    // TODO: variants
    SetSignatureMode { enable: bool } = 0,
    AddExtension { account_id: AccountId } = 1,
    RemoveExtension { account_id: AccountId } = 2,
}
