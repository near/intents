#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{borrow::Cow, collections::{BTreeMap, HashSet}};

use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
pub use defuse_deadline::Deadline;
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Promise, borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};
use thiserror::Error as ThisError;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Upgrade {
    pub approved_by: AccountId,
    #[borsh(
        serialize_with = "As::<TimestampNanoSeconds>::serialize",
        deserialize_with = "As::<TimestampNanoSeconds>::deserialize",
    )]
    pub valid_by: Deadline,
    pub old_hashes: HashSet<[u8; 32]>,
    pub whitelisted_executors: Option<HashSet<AccountId>>,
    pub whitelisted_revokers: Option<HashSet<AccountId>>,
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtraParams {
    pub approvals: BTreeMap<[u8; 32], Upgrade>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleanupInfo {
    pub expired_removed: u32,
    pub stale_owner_removed: u32,
}

pub type ApproveResult = (AsHex<[u8; 32]>, CleanupInfo);

pub type RevokeResult = (u32, CleanupInfo);

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("HashIsAlreadyApproved {0:?}")]
    HashIsAlreadyApproved([u8; 32]),
    #[error("ApprovedUpgradesCountExceeded {0}")]
    ApprovedUpgradesCountExceeded(usize),
    #[error("HashNotFound {0:?}")]
    HashNotFound([u8; 32]),
    #[error("unauthorized")]
    Unauthorized,
    #[error("approval expired")]
    Expired,
    #[error("current code hash not in approved old_hashes")]
    OldHashMismatch,
    #[error("old_hashes must not be empty")]
    EmptyOldHashes,
    #[error("new_code hash does not match approved new_hash")]
    NewCodeHashMismatch,
}

impl ExtraParams {
    pub const STORAGE_KEY: &[u8] = b"upgrade";
    pub const MAX_PENDING_APPROVALS: usize = 32;

    fn load() -> Self {
        near_sdk::env::storage_read(Self::STORAGE_KEY)
            .map(|bytes| borsh::from_slice(&bytes).unwrap_or_default())
            .unwrap_or_default()
    }

    fn save(&self) {
        near_sdk::env::storage_write(
            Self::STORAGE_KEY,
            &borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        );
    }

    fn cleanup(&mut self, owner: &AccountId) -> CleanupInfo {
        let mut info = CleanupInfo::default();
        self.approvals.retain(|_hash, upgrade| {
            if upgrade.valid_by.has_expired() {
                info.expired_removed += 1;
                return false;
            }
            if &upgrade.approved_by != owner {
                info.stale_owner_removed += 1;
                return false;
            }
            true
        });
        info
    }

    pub fn approve(
        new_hash: [u8; 32],
        upgrade: Upgrade,
        owner: &AccountId,
    ) -> Result<(AsHex<[u8; 32]>, CleanupInfo), Error> {
        let mut params = Self::load();
        let cleanup_info = params.cleanup(owner);

        if params.approvals.contains_key(&new_hash) {
            return Err(Error::HashIsAlreadyApproved(new_hash));
        }
        if upgrade.old_hashes.is_empty() {
            return Err(Error::EmptyOldHashes);
        }
        if params.approvals.len() >= Self::MAX_PENDING_APPROVALS {
            return Err(Error::ApprovedUpgradesCountExceeded(
                Self::MAX_PENDING_APPROVALS,
            ));
        }

        params.approvals.insert(new_hash, upgrade);
        params.save();

        Ok((new_hash.into(), cleanup_info))
    }

    pub fn revoke(
        hashes: &[[u8; 32]],
        caller: &AccountId,
        owner: &AccountId,
    ) -> Result<RevokeResult, Error> {
        let mut params = Self::load();
        let cleanup_info = params.cleanup(owner);
        let mut revoked = 0u32;

        for hash in hashes {
            let Some(upgrade) = params.approvals.get(hash) else {
                return Err(Error::HashNotFound(*hash));
            };
            let authorized = caller == owner
                || upgrade
                    .whitelisted_revokers
                    .as_ref()
                    .is_some_and(|set| set.contains(caller));
            if !authorized {
                return Err(Error::Unauthorized);
            }
            params.approvals.remove(hash);
            revoked += 1;
        }

        params.save();
        Ok((revoked, cleanup_info))
    }

    pub fn check(
        new_hash: &[u8; 32],
        caller: &AccountId,
        owner: &AccountId,
        current_code_hash: &[u8; 32],
    ) -> Result<CleanupInfo, Error> {
        let mut params = Self::load();
        let cleanup_info = params.cleanup(owner);

        let upgrade = params
            .approvals
            .get(new_hash)
            .ok_or(Error::HashNotFound(*new_hash))?;

        let authorized = caller == owner
            || upgrade
                .whitelisted_executors
                .as_ref()
                .is_some_and(|set| set.contains(caller));
        if !authorized {
            return Err(Error::Unauthorized);
        }
        if !upgrade.old_hashes.contains(current_code_hash) {
            return Err(Error::OldHashMismatch);
        }
        if upgrade.valid_by.has_expired() {
            return Err(Error::Expired);
        }

        params.save();
        Ok(cleanup_info)
    }

    pub fn take(new_hash: &[u8; 32]) -> Result<Upgrade, Error> {
        let mut params = Self::load();
        let upgrade = params
            .approvals
            .remove(new_hash)
            .ok_or(Error::HashNotFound(*new_hash))?;
        params.save();
        Ok(upgrade)
    }
}

/// Manages global contract code and ownership for deterministic (NEP-616) accounts.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Approves a future upgrade by hash.
    /// Owner-only. Returns the approved hash and cleanup info.
    /// Emits [`Event::Approve`].
    fn gd_approve(
        &mut self,
        new_hash: AsHex<[u8; 32]>,
        old_hashes: Vec<AsHex<[u8; 32]>>,
        valid_by: Deadline,
        whitelisted_executors: Option<HashSet<AccountId>>,
        whitelisted_revokers: Option<HashSet<AccountId>>,
    ) -> ApproveResult;

    /// Revokes previously approved upgrades by hash.
    /// Caller must be owner or whitelisted revoker for each entry.
    /// Emits [`Event::Revoke`].
    fn gd_revoke(&mut self, hashes: Vec<AsHex<[u8; 32]>>) -> RevokeResult;

    /// Executes a previously approved upgrade.
    /// Caller must be owner or whitelisted executor.
    /// Requires attached deposit for storage.
    fn gd_execute_upgrade(
        &mut self,
        #[serializer(borsh)] new_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise;

    /// Deploys WASM code as a global contract on this account.
    /// - code: WASM code to deploy.
    /// - old_hash: hash of the currently deployed code or `[State::DEFAULT_HASH]` on first use.
    /// Requires attached deposit for storage and owner-only access.
    /// Emits [`Event::Deploy`].
    fn gd_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise;

    /// Transfers contract ownership to `receiver_id`.
    /// Requires 1 yoctoNEAR, owner-only, no self-transfer.
    /// Emits [`Event::Transfer`].
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    /// Returns the current owner's account ID.
    fn gd_owner_id(&self) -> AccountId;

    /// Returns the deployer instance index (used for deterministic account derivation).
    fn gd_index(&self) -> u32;

    /// Returns the SHA-256 hash of the currently deployed code, or `0000..000` if none.
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;
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
    Approve {
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Revoke {
        #[serde_as(as = "Vec<Hex>")]
        hashes: Vec<[u8; 32]>,
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
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0; 32];

    pub fn new(owner: impl Into<AccountId>, index: u32) -> Self {
        Self {
            owner_id: owner.into(),
            index,
            code_hash: Self::DEFAULT_HASH,
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
