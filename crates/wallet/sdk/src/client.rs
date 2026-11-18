//! Bindings to [`Wallet`](defuse_wallet::contract::Wallet) contract

use std::collections::BTreeSet;

use defuse_wallet::{Request, RequestMessage, Timestamp};
use near_kit::{AccountId, AccountIdRef, NearToken};

#[near_kit::contract]
pub trait WalletContract {
    /// See [`w_execute_signed()`](defuse_wallet::contract::Wallet::w_execute_signed) method.
    #[call]
    fn w_execute_signed(&mut self, msg: &RequestMessage, proof: &str);

    /// See [`w_execute_extension()`](defuse_wallet::contract::Wallet::w_execute_extension) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn w_execute_extension(&mut self, request: &Request);

    /// See [`w_subwallet_id()`](defuse_wallet::contract::Wallet::w_subwallet_id) method.
    fn w_subwallet_id(&self) -> u32;

    /// See [`w_is_signature_allowed()`](defuse_wallet::contract::Wallet::w_is_signature_allowed) method.
    fn w_is_signature_allowed(&self) -> bool;

    /// See [`w_public_key()`](defuse_wallet::contract::Wallet::w_public_key) method.
    fn w_public_key(&self) -> String;

    /// See [`w_is_extension_enabled()`](defuse_wallet::contract::Wallet::w_is_extension_enabled) method.
    fn w_is_extension_enabled(&self, account_id: &AccountIdRef) -> bool;

    /// See [`w_extensions()`](defuse_wallet::contract::Wallet::w_extensions) method.
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// See [`w_timeout_secs()`](defuse_wallet::contract::Wallet::w_timeout_secs) method.
    fn w_timeout_secs(&self) -> u32;

    /// See [`w_last_cleaned_at()`](defuse_wallet::contract::Wallet::w_last_cleaned_at) method.
    fn w_last_cleaned_at(&self) -> Timestamp;
}
