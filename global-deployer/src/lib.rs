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
    Deploy(#[serde_as(as = "Hex")] CryptoHash),

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
    /// Requires attached deposit for storage and owner-only access.
    /// Emits [`Event::Deploy`]. Refunds deposit on failure.
    fn gd_deploy(&mut self, #[serializer(borsh)] code: Vec<u8>, #[serializer(borsh)] old_hash: [u8; 32]) -> Promise;

    /// Transfers contract ownership to `receiver_id`.
    /// Requires 1 yoctoNEAR, owner-only, no self-transfer.
    /// Emits [`Event::Transfer`].
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    /// Returns the current owner's account ID.
    fn gd_owner_id(&self) -> AccountId;

    /// Returns the deployer instance index (used for deterministic account derivation).
    fn gd_index(&self) -> u32;
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub owner_id: AccountId,
    pub index: u32,
    #[serde_as(as = "Hex")]
    pub code_hash: CryptoHash,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: CryptoHash = [0; 32];

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
