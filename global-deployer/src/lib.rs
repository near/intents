use std::collections::BTreeMap;

use near_sdk::serde_with::{hex::Hex, serde_as};
use near_sdk::{AccountId, CryptoHash, PanicOnDefault, Promise, borsh, ext_contract, near};

#[cfg(feature = "contract")]
mod contract;
pub mod error;

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "global-deployer", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "global-deployer"))]
pub enum Event {
    #[event_version("1.0.0")]
    Deploy {
        #[serde_as(as = "Hex")]
        old_hash: CryptoHash,
        #[serde_as(as = "Hex")]
        new_hash: CryptoHash,
    },

    #[event_version("1.0.0")]
    Transfer {
        old_owner_id: AccountId,
        new_owner_id: AccountId,
    },
}

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
    /// Refunds
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

    /// Returns the SHA-256 hash of the currently deployed code, or `[0; 32]` if none.
    fn gd_code_hash(&self) -> [u8; 32];
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
            borsh::to_vec(&self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
