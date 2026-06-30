#![doc = include_str!(env!("CARGO_PKG_README"))]

#[cfg(feature = "contract")]
mod contract;
mod error;
mod events;
pub mod signature;

pub use self::{error::*, events::*};

use std::collections::BTreeSet;

pub use defuse_wallet_core::*;
use near_sdk::ext_contract;

// TODO: separate traits
/// Deterministic single-key Wallet Contract.
#[ext_contract(ext_wallet)]
pub trait Wallet {
    /// Execute signed request message.
    ///
    /// SHOULD accept ANY attached deposit.
    ///
    /// MUST fail in case where the `msg.request` was not executed
    /// due to various reasons, including:
    ///   * `msg` data is invalid
    ///   * `proof` is invalid
    ///   * signature is disabled
    ///   * nonce is already used
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String);

    /// Execute request from an enabled extension.
    ///
    /// * SHOULD accept ANY **non-zero** attached deposit
    /// * MUST panic if zero deposit was attached
    /// * MUST panic if [`predecessor_account_id`](near_sdk::env::predecessor_account_id)
    ///   extension is not enabled
    fn w_execute_extension(&mut self, request: Request);

    /// Returns `subwallet_id`.
    fn w_subwallet_id(&self) -> u32;

    /// Returns whether authentication by signature is currently allowed.
    fn w_is_signature_allowed(&self) -> bool;

    /// Returns a string representation of the public key or authentication
    /// identity associated with this wallet's singing standard.
    fn w_public_key(&self) -> String;

    /// Returns whether extension with given `account_id` is enabled.
    /// If true, this `account_id` SHOULD be allowed to call
    /// `w_execute_extension()`.
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of enabled extensions. Each returned account
    /// SHOULD be allowed to call `w_execute_extension()`.
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// Returns a timeout, i.e. validity timespan for each nonce.
    fn w_timeout_secs(&self) -> u32;

    /// Returns a timestamp when nonces were last cleaned up.
    fn w_last_cleaned_at(&self) -> Timestamp;
}
