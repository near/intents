#[cfg(any(feature = "arbitrary", test))]
mod arbitrary;
#[cfg(feature = "contract")]
mod contract;
mod error;
mod events;
mod request;
pub mod signature;
mod state;

use std::collections::BTreeSet;

use near_sdk::{AccountId, ext_contract};

use crate::signature::RequestMessage;

pub use self::{error::*, events::*, request::*, state::*};

/// Deterministic single-key Wallet Contract.
#[ext_contract(ext_wallet)]
pub trait Wallet {
    /// Executes signed request.
    ///
    /// * SHOULD accept ANY attached deposit.
    /// * MUST fail in any case where the `signed.request` is not executed
    ///   due to various reasons, including:
    ///   * `signed` data is invalid
    ///   * `proof` is invalid
    ///   * signature is disabled
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String);

    /// Execute request from an enabled extension.
    ///
    /// * SHOULD accept ANY non-zero attached deposit
    /// * MUST panic if zero deposit was attached
    /// * MUST panic if [`predecessor_account_id`](near_sdk::env::predecessor_account_id)
    ///   extension is not enabled
    fn w_execute_extension(&mut self, request: Request);

    /// Returns subwallet_id.
    fn w_subwallet_id(&self) -> u32;

    /// Returns whether authentication by signature is currently allowed.
    fn w_is_signature_allowed(&self) -> bool;

    /// Returns a string representation of the public key or authentication
    /// identity associated with this wallet's singing standard.
    fn w_public_key(&self) -> String;

    /// Current `seqno` to be used for signed requests.
    fn w_seqno(&self) -> u32;

    /// Returns whether extension with given `account_id` is enabled.
    /// If true, this `account_id` SHOULD be allowed to call
    /// `w_execute_extension()`.
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of enabled extensions. Each returned account
    /// SHOULD be allowed to call `w_execute_extension()`.
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// Helper method to get chain_id of the network
    fn w_chain_id(&self) -> String;
}
