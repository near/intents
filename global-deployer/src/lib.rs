#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Promise, borsh, ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};
use thiserror::Error as ThisError;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    pub owner_id: AccountId,
    #[serde_as(as = "Hex")]
    pub new_hash: [u8; 32],
    #[serde_as(as = "BTreeSet<Hex>")]
    pub old_hashes: BTreeSet<[u8; 32]>,
    pub whitelisted_executors: BTreeSet<AccountId>,
    pub whitelisted_revokers: BTreeSet<AccountId>,
}

impl Deployment {
    pub fn hash(&self) -> [u8; 32] {
        near_sdk::env::sha256_array(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApprovedDeployments(pub BTreeSet<[u8; 32]>);

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("HashIsAlreadyApproved {0:?}")]
    HashIsAlreadyApproved([u8; 32]),
    #[error("HashNotFound {0:?}")]
    HashNotFound([u8; 32]),
    #[error("unauthorized")]
    Unauthorized,
    #[error("current code hash not in approved old_hashes")]
    OldHashMismatch,
    #[error("old_hashes must not be empty")]
    EmptyOldHashes,
    #[error("new_code hash does not match approved new_hash")]
    NewCodeHashMismatch,
    #[error("deployment.owner_id does not match current contract owner")]
    OwnerMismatch,
    #[error("contract owner changed since approval")]
    OwnerChanged,
}

impl ApprovedDeployments {
    pub const STORAGE_KEY: &[u8] = b"approved";
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

    pub fn approve(deployment: &Deployment, current_owner_id: &AccountId) -> Result<(), Error> {
        if deployment.owner_id != *current_owner_id {
            return Err(Error::OwnerMismatch);
        }
        if deployment.old_hashes.is_empty() {
            return Err(Error::EmptyOldHashes);
        }

        let mut params = Self::load();
        let deployment_hash = deployment.hash();

        if !params.0.insert(deployment_hash) {
            return Err(Error::HashIsAlreadyApproved(deployment_hash));
        }
        params.save();

        Ok(())
    }

    pub fn revoke(
        deployments: &[Deployment],
        caller: &AccountId,
        current_owner_id: &AccountId,
    ) -> Result<(), Error> {
        let mut params = Self::load();

        for deployment in deployments {
            let deployment_hash = deployment.hash();

            if !params.0.contains(&deployment_hash) {
                return Err(Error::HashNotFound(deployment_hash));
            }

            let owner_changed = deployment.owner_id != *current_owner_id;
            let authorized = owner_changed
                || caller == current_owner_id
                || caller == &deployment.owner_id
                || deployment.whitelisted_revokers.contains(caller);
            if !authorized {
                return Err(Error::Unauthorized);
            }
            params.0.remove(&deployment_hash);
        }

        params.save();
        Ok(())
    }

    pub fn check(
        deployment: &Deployment,
        caller: &AccountId,
        current_owner_id: &AccountId,
        current_code_hash: &[u8; 32],
        actual_code_hash: &[u8; 32],
    ) -> Result<(), Error> {
        if deployment.new_hash != *actual_code_hash {
            return Err(Error::NewCodeHashMismatch);
        }

        let params = Self::load();
        let deployment_hash = deployment.hash();

        if !params.0.contains(&deployment_hash) {
            return Err(Error::HashNotFound(deployment_hash));
        }

        if &deployment.owner_id != current_owner_id {
            return Err(Error::OwnerChanged);
        }

        let authorized =
            caller == &deployment.owner_id || deployment.whitelisted_executors.contains(caller);
        if !authorized {
            return Err(Error::Unauthorized);
        }
        if !deployment.old_hashes.contains(current_code_hash) {
            return Err(Error::OldHashMismatch);
        }

        Ok(())
    }

    pub fn take(deployment_hash: &[u8; 32]) -> Result<(), Error> {
        let mut params = Self::load();
        if !params.0.remove(deployment_hash) {
            return Err(Error::HashNotFound(*deployment_hash));
        }
        params.save();
        Ok(())
    }
}

/// Manages global contract code and ownership for deterministic (NEP-616) accounts.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Approves a future deployment by hash.
    /// Owner-only. Returns the approved hash and cleanup info.
    /// Emits [`Event::DeploymentApproved`].
    fn gd_approve(&mut self, deployment: Deployment);

    /// Revokes previously approved deployments by hash.
    /// Caller must be owner or whitelisted revoker for each entry.
    /// Emits [`Event::DeploymentsRevoked`].
    fn gd_revoke(&mut self, deployments: Vec<Deployment>);

    /// Executes a previously approved deployment.
    /// Caller must be owner or whitelisted executor.
    /// Requires attached deposit for storage.
    fn gd_exec_approved_deployment(
        &mut self,
        #[serializer(borsh)] deployment: Deployment,
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

    /// Converts a JSON-serialized Deployment to borsh. Useful for constructing
    /// `gd_exec_approved_deployment` calls from Deployment data found in block explorers.
    fn gd_as_borsh(&self, deployment: Deployment) -> Vec<u8>;
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
        deployment_hash: [u8; 32],
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    DeploymentsRevoked {
        #[serde_as(as = "Vec<Hex>")]
        hashes: Vec<[u8; 32]>,
    },

    #[event_version("1.0.0")]
    ApprovedDeploymentExecuted {
        #[serde_as(as = "Hex")]
        deployment_hash: [u8; 32],
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
