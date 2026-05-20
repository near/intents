use near_sdk::{AccountId, near};

/// An operation to perform on wallet-contract's state.
///
/// # Idempotence
///
/// Currently, all operations are idempotent, i.e. they express an intent on
/// the final state rather than "compare-and-swap" semantics. If ever needed,
/// we can add more `require_*` operations, e.g.
/// `RequireExtensionEnabled { account_id }`.
///
/// We could have added an optional `strict` flag to each operation. However,
/// it would not only break borsh serialization, but it's also less expressive
/// than potential `require_*` variants: one might want to only atomically
/// require the contract to be in a specific state, without mutating it.
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
