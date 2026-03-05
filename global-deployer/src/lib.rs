#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{borrow::Cow, collections::BTreeMap};

use defuse_borsh_utils::adapters::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Promise, borsh, ext_contract, near,
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

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Owner's account ID.
    pub owner_id: AccountId,
    /// Currently deployed code hash, or zeros otherwise.
    #[serde_as(as = "Hex")]
    pub code_hash: [u8; 32],
    /// Hash of the approved next deployment, or zeros if none.
    #[serde_as(as = "Hex")]
    pub approved_hash: [u8; 32],
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0; 32];

    pub fn new(owner: impl Into<AccountId>) -> Self {
        Self {
            owner_id: owner.into(),
            code_hash: Self::DEFAULT_HASH,
            approved_hash: Self::DEFAULT_HASH,
        }
    }

    #[must_use]
    pub fn with_index(mut self, index: u32) -> Self {
        let mut hash = [0u8; 32];
        hash[32 - 4..].copy_from_slice(&index.to_be_bytes());
        self.code_hash = hash;
        self
    }

    #[must_use]
    pub const fn pre_approve(mut self, hash: [u8; 32]) -> Self {
        self.approved_hash = hash;
        self
    }

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
