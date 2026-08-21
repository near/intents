#[cfg(feature = "contract")]
pub mod contract;

use std::collections::HashMap;

use defuse_admin_utils::full_access_keys::FullAccessKeys;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_plugins::AccessControllable;
use near_sdk::{
    AccountId, Promise, ext_contract,
    json_types::{Base64VecU8, U128},
    near,
};

/// Metadata about a cross-chain withdrawal tracked by the factory.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone)]
pub struct Withdrawal {
    pub chain_id: String,
    pub payload_hash: Base64VecU8,
    pub timestamp: u64,
    pub metadata: String,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "factory"))]
#[derive(Debug, Clone)]
pub enum FactoryEvent<'a> {
    #[event_version("0.1.0")]
    FtWithdraw {
        withdrawal_id: &'a str,
        withdrawal: &'a Withdrawal,
    },
    #[event_version("0.1.0")]
    FtUpdateWithdraw {
        withdrawal_id: &'a str,
        prev_payload_hash: &'a Base64VecU8,
        new_payload_hash: &'a Base64VecU8,
        metadata: &'a String,
    },
}

#[ext_contract(ext_poa_factory)]
pub trait PoaFactory: AccessControllable + FullAccessKeys {
    /// Deploys new token to `token.<CURRENT_ACCOUNT_ID>`.
    /// Requires to attach enough Ⓝ to cover storage costs.
    fn deploy_token(&mut self, token: String, metadata: Option<FungibleTokenMetadata>) -> Promise;

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

    /// Same as [`PoaFactory::ft_deposit`], but deduplicates deposits by
    /// `deposit_id`: the id is recorded on-chain and the call fails if it was
    /// already used. Required for omni layer tokens, which [`PoaFactory::ft_deposit`]
    /// rejects.
    ///
    /// NOTE: the `deposit_id` entry is not covered by the attached deposit,
    /// so its storage MUST be subsidised separately by the contract owner.
    fn ft_omni_deposit(
        &mut self,
        deposit_id: String,
        token: String,
        owner_id: AccountId,
        amount: U128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Promise;

    /// Returns a mapping of token names to their account ids.
    fn tokens(&self) -> HashMap<String, AccountId>;

    /// Records a new withdrawal under `withdrawal_id`. Fails if the id is already used.
    ///
    /// NOTE: as with [`PoaFactory::ft_omni_deposit`], this storage MUST be
    /// subsidised separately by the contract owner.
    fn ft_withdraw(&mut self, withdrawal_id: String, withdrawal: Withdrawal);

    /// Replaces the payload hash of an existing withdrawal, guarded by the previous hash.
    fn ft_update_withdraw(
        &mut self,
        withdrawal_id: String,
        prev_payload_hash: Base64VecU8,
        new_payload_hash: Base64VecU8,
        metadata: String,
    );

    /// Returns the withdrawal stored under `withdrawal_id`, if any.
    fn get_withdraw(&self, withdrawal_id: String) -> Option<&Withdrawal>;

    /// Removes the given withdrawal ids from storage.
    fn remove_withdraws(&mut self, withdrawals: Vec<String>);

    /// Removes the given deposit ids from storage, allowing them to be reused.
    fn remove_deposits(&mut self, deposits: Vec<String>);

    /// Adds the given tokens to the list of omni layer tokens.
    fn add_omni_tokens(&mut self, tokens: Vec<String>);
    /// Removes the given tokens from the list of omni layer tokens.
    fn remove_omni_tokens(&mut self, tokens: Vec<String>);
    /// Returns the list of omni layer tokens.
    fn get_omni_tokens(&self) -> Vec<String>;
}
