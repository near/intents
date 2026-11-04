//! Bindings to [`Wallet`](defuse_wallet::contract::Wallet) contract

use std::collections::BTreeSet;

use defuse_wallet::{Request, RequestMessage, Timestamp};
use near_kit::{AccountId, AccountIdRef, NearToken};

#[near_kit::contract]
pub trait WalletContract {
    #[call]
    fn w_execute_signed(&mut self, msg: &RequestMessage, proof: &str);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn w_execute_extension(&mut self, request: &Request);

    fn w_subwallet_id(&self) -> u32;

    fn w_is_signature_allowed(&self) -> bool;

    fn w_public_key(&self) -> String;

    fn w_is_extension_enabled(&self, account_id: &AccountIdRef) -> bool;

    fn w_extensions(&self) -> BTreeSet<AccountId>;

    fn w_timeout_secs(&self) -> u32;

    fn w_last_cleaned_at(&self) -> Timestamp;
}
