use near_sdk::{AccountId, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WalletOp {
    /// Enable or disable authentication by signature.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// signature mode is already in this state.
    SetSignatureMode { enable: bool } = 0,

    /// Add extension with given `AccountId`.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// given extension is already enabled.
    AddExtension { account_id: AccountId } = 1,

    /// Remove extension with given `AccountId`.
    ///
    /// This operation is idempotent, i.e. no error will be raised if
    /// given extension is not currently enabled.
    RemoveExtension { account_id: AccountId } = 2,
}
