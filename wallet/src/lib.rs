#[cfg(feature = "contract")]
mod contract;
mod error;
mod request;
mod signature;
mod state;
mod utils;

use std::collections::BTreeSet;

use near_sdk::{AccountId, ext_contract};

pub use self::{error::*, request::*, signature::*, state::*};

#[ext_contract(ext_wallet)]
pub trait Wallet {
    /// Executes signed request.
    ///
    /// TODO: The wallet-contract MIGHT have some whitelists
    fn w_execute_signed(&mut self, signed: SignedRequest, proof: String);

    // TODO: accept query_id?
    /// Execute request from an enabled extension.
    ///
    /// * MUST panic if [`predecessor_account_id`](near_sdk::env::predecessor_account_id)
    /// is not an enabled extension.
    /// * MUST be `#[payable]` and accept ANY attached deposit
    fn w_execute_extension(&mut self, request: Request);

    /// Returns subwallet_id
    fn w_subwallet_id(&self) -> u32;

    /// Returns whether authentication by signature is allowed.
    fn w_is_signature_allowed(&self) -> bool;

    // TODO: view-method to get supported signing standard?
    // TODO: answer: it can be retrieved via contract_source_metadata

    // TODO: OAuth 2.0, off-chain multisig, TEE
    /// Returns whether authentication by signature is allowed.
    fn w_public_key(&self) -> String;

    fn w_seqno(&self) -> u32;

    /// Returns whether extension with given `account_id` is enabled.
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of enabled extensions.
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// Returns chain_id of the network
    fn w_chain_id(&self) -> String;
}
