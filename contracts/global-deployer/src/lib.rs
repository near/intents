#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::borrow::Cow;

pub use defuse_borsh_utils::adapters::{AsWrap, Remainder};
pub use defuse_global_deployer_core::State;
pub use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Promise, ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};

/// Manages global contract code and ownership for deterministic (NEP-616) accounts.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Approves a future deployment by setting the expected new code hash.
    /// If an approved hash was already set, it will be replaced.
    /// Owner-only. Requires 1 yoctoNEAR. Verifies `old_hash` matches current `code_hash`.
    /// Emits [`Event::Approve`] with [`Reason::By`].
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>);

    /// Deploys WASM code as a global contract on this account.
    /// Permissionless: anyone can call if `sha256(code)` matches `approved_hash`.
    /// Requires attached deposit for storage.
    /// Emits [`Event::Deploy`] and [`Event::Approve`] with [`Reason::Deploy`].
    ///
    /// The `code` argument accepts raw `.wasm` bytes directly — just pass the
    /// binary contents of the file as the function call input. There is no need
    /// to prepend a borsh length prefix; the [`AsWrap`]`<`[`Vec<u8>`]`,
    /// `[`Remainder`]`>` adapter consumes all input bytes as-is.
    fn gd_deploy(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>) -> Promise;

    /// Transfers contract ownership to `receiver_id`.
    /// Resets `approved_hash` to `DEFAULT_HASH`.
    /// Requires 1 yoctoNEAR, owner-only, no self-transfer.
    /// Emits [`Event::Transfer`] and [`Event::Approve`] with [`Reason::By`].
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    /// Returns the current owner's account ID.
    fn gd_owner_id(&self) -> AccountId;

    /// Returns the SHA-256 hash of the currently deployed code, or `0000..000` if none.
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the currently approved hash, or `0000..000` if none.
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "global-deployer"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    Approve {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
        reason: Reason<'a>,
    },

    #[event_version("1.0.0")]
    Deploy {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Transfer {
        old_owner_id: Cow<'a, AccountIdRef>,
        new_owner_id: Cow<'a, AccountIdRef>,
    },
}

#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason<'a> {
    Deploy(#[serde_as(as = "Hex")] [u8; 32]),
    By(Cow<'a, AccountIdRef>),
}
