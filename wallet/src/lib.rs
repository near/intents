#[cfg(any(feature = "arbitrary", test))]
mod arbitrary;
#[cfg(feature = "contract")]
mod contract;
mod error;
mod events;
mod nonces;
mod request;
pub mod signature;
mod state;

use std::collections::BTreeSet;

use defuse_deadline::Deadline;
use near_sdk::{AccountId, ext_contract};

use crate::signature::RequestMessage;

pub use self::{error::*, events::*, nonces::*, request::*, state::*};

/// Deterministic single-key Wallet Contract.
#[ext_contract(ext_wallet)]
pub trait Wallet {
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

    // TODO: docs
    fn w_timeout_sec(&self) -> u32;
    fn w_last_cleaned_at(&self) -> Deadline;

    /// Returns whether extension with given `account_id` is enabled.
    /// If true, this `account_id` SHOULD be allowed to call
    /// `w_execute_extension()`.
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of enabled extensions. Each returned account
    /// SHOULD be allowed to call `w_execute_extension()`.
    fn w_extensions(&self) -> BTreeSet<AccountId>;
}
