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
/// An operation to perform on wallet-contract's state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WalletOp {
    /// Enable or disable authentication by signature.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// signature mode is already in this state.
    SetSignatureMode { enable: bool } = 0,

    /// Add an extension with given `AccountId`.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// given extension is already enabled.
    AddExtension { account_id: AccountId } = 1,

    /// Remove an extension with given `AccountId`.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// given extension is not currently enabled.
    RemoveExtension { account_id: AccountId } = 2,
}
