#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{borrow::Cow, collections::BTreeMap};

use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Promise, borsh, ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};

/// Manages global contract code and ownership for deterministic (NEP-616) accounts.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Approves a future deployment by setting the expected new code hash.
    /// Owner-only. Verifies `old_hash` matches current `code_hash`.
    /// Emits [`Event::DeploymentApproved`].
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>);

    /// Deploys WASM code as a global contract on this account.
    /// Permissionless: anyone can call if `old_hash` matches current `code_hash`
    /// and `sha256(new_code)` matches `approved_hash`.
    /// Owner can deploy without prior approval.
    /// Requires attached deposit for storage.
    /// Emits [`Event::Deploy`].
    fn gd_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise;

    /// Transfers contract ownership to `receiver_id`.
    /// Resets `approved_hash` to `DEFAULT_HASH`.
    /// Requires 1 yoctoNEAR, owner-only, no self-transfer.
    /// Emits [`Event::Transfer`].
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    /// Returns the current owner's account ID.
    fn gd_owner_id(&self) -> AccountId;

    /// Returns the deployer instance index (used for deterministic account derivation).
    fn gd_index(&self) -> u32;

    /// Returns the SHA-256 hash of the currently deployed code, or `0000..000` if none.
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the currently approved hash, or `0000..000` if none.
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "global-deployer"))]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    Deploy {
        #[serde_as(as = "Hex")]
        old_hash: [u8; 32],
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Transfer {
        old_owner_id: Cow<'a, AccountIdRef>,
        new_owner_id: Cow<'a, AccountIdRef>,
    },

    #[event_version("1.0.0")]
    DeploymentApproved {
        #[serde_as(as = "Hex")]
        old_hash: [u8; 32],
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Owner's account ID.
    pub owner_id: AccountId,
    /// Deployed instance index
    pub index: u32,
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

    pub fn new(owner: impl Into<AccountId>, index: u32) -> Self {
        Self {
            owner_id: owner.into(),
            index,
            code_hash: Self::DEFAULT_HASH,
            approved_hash: Self::DEFAULT_HASH,
        }
    }

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
