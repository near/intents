use near_account_id::AccountId;

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(tag = "op", content = "payload", rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
/// An internal operation to perform on wallet-contract's state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WalletOp {
    /// Enable or disable authentication by signature.
    ///
    /// If signature mode is already in specified state, the contract MUST panic.
    SetSignatureMode { enable: bool } = 0,

    /// Add an extension with given `AccountId`.
    ///
    /// If this extension is already enabled, the contract MUST panic.
    AddExtension { account_id: AccountId } = 1,

    /// Remove an extension with given `AccountId`.
    ///
    /// If this extension is not currently enabled, the contract MUST panic.
    RemoveExtension { account_id: AccountId } = 2,
}
