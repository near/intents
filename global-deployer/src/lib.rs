#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{borrow::Cow, collections::BTreeMap};

use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, GlobalContractId, Promise, borsh, ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};

/// Manages global contract code and ownership for deterministic (NEP-616) accounts.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Deploys WASM code as a global contract on this account.
    /// - code: WASM code to deploy.
    /// - old_hash: hash of the currently deployed code or `[State::DEFAULT_HASH]` on first use.
    /// Requires attached deposit for storage and owner-only access.
    /// Emits [`Event::Deploy`].
    /// If a returned promise succeeds, then it means the new code was successfully
    /// deployed globally on the account_id of this deployer contract.
    /// If current or returned promise fails, then it means that the new code was not deployed.
    /// Refunds to `refund_to` set on the current receipt (`predecessor_id` by default):
    /// - excessive deposit on success
    /// - attached deposit on failure
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
    /// Owner's contract ID derived from ([`Sate::owner_id`],[`OwnerProxyState`]).
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub owner_contract_id: Option<GlobalContractId>,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0; 32];

    pub fn new(owner: impl Into<AccountId>, index: u32) -> Self {
        Self {
            owner_id: owner.into(),
            index,
            code_hash: Self::DEFAULT_HASH,
            owner_contract_id: None,
        }
    }

    pub fn new_with_contract(
        owner: impl Into<AccountId>,
        owner_contract_id: GlobalContractId,
        index: u32,
    ) -> Self {
        Self {
            owner_id: owner.into(),
            index,
            code_hash: Self::DEFAULT_HASH,
            owner_contract_id: Some(owner_contract_id),
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

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerProxyState {
    /// Owner's account ID should match [`State::owner_id`].
    pub owner_id: AccountId,
    /// Currently deployed code hash
    #[serde_as(as = "Hex")]
    pub old_hash: [u8; 32],
    /// New deployed code hash
    #[serde_as(as = "Hex")]
    pub new_hash: [u8; 32],
    /// Identifies exact deployer instance
    pub deployer_instance: AccountId,
}

impl OwnerProxyState {
    pub const STATE_KEY: &[u8] = b"";

    pub const fn new(
        owner_id: AccountId,
        old_hash: [u8; 32],
        new_hash: [u8; 32],
        deployer_instance: AccountId,
    ) -> Self {
        Self {
            owner_id,
            old_hash,
            new_hash,
            deployer_instance,
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
