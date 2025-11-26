#[cfg(feature = "contract")]
pub mod contract;

use std::collections::HashMap;

use defuse_admin_utils::full_access_keys::FullAccessKeys;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_plugins::AccessControllable;
use near_sdk::{AccountId, Promise, PublicKey, ext_contract, json_types::U128};

pub trait TokenFullAccessKeys {
    /// Adds a full access key to the given token contract.
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn add_token_full_access_key(&mut self, token: String, public_key: PublicKey) -> Promise;

    /// Deletes a full access key from the given token contract.
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn delete_token_full_access_key(&mut self, token: String, public_key: PublicKey) -> Promise;
}

#[ext_contract(ext_poa_factory)]
pub trait PoaFactory: AccessControllable + FullAccessKeys + TokenFullAccessKeys {
    /// Deploys new token to `token.<CURRENT_ACCOUNT_ID>`.
    /// Requires to attach enough Ⓝ to cover storage costs.
    fn deploy_token(
        &mut self,
        token: String,
        metadata: Option<FungibleTokenMetadata>,
        no_registration: Option<bool>,
    ) -> Promise;

    /// Sets metadata on `token.<CURRENT_ACCOUNT_ID>`.
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn set_metadata(&mut self, token: String, metadata: FungibleTokenMetadata) -> Promise;

    /// Deposits `token.<CURRENT_ACCOUNT_ID>` for `owner_id` by forwarding it
    /// to `token_id::ft_deposit(owner_id, amount, memo)` or
    // `token_id::ft_transfer_call(owner_id, amount, msg, memo)` if msg is given.
    /// Requires to attach enough Ⓝ to cover storage costs.
    fn ft_deposit(
        &mut self,
        token: String,
        owner_id: AccountId,
        amount: U128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Promise;

    /// Returns a mapping of token names to their account ids.
    fn tokens(&self) -> HashMap<String, AccountId>;
}
