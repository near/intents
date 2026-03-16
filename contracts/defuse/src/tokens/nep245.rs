#![allow(clippy::too_many_arguments)]

use defuse_nep245::{MultiTokenCore, TokenId, receiver::MultiTokenReceiver};
use near_plugins::AccessControllable;
use near_sdk::{AccountId, PromiseOrValue, ext_contract, json_types::U128};

#[ext_contract(ext_mt_withdraw)]
pub trait MultiTokenWithdrawer: MultiTokenReceiver + MultiTokenWithdrawResolver {
    /// Returns number of tokens were successfully withdrawn
    ///
    /// Optionally can specify `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn mt_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
}

#[ext_contract(mt_withdraw_resolver)]
pub trait MultiTokenWithdrawResolver {
    fn mt_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        is_call: bool,
    ) -> Vec<U128>;
}

/// Same as [`MultiTokenCore`], but allows permissioned accounts to transfer
/// from any `owner_id` bypassing locked checks.
#[ext_contract(ext_mt_force_core)]
pub trait MultiTokenForcedCore: MultiTokenCore + AccessControllable {
    fn mt_force_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    );

    fn mt_force_batch_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    );

    fn mt_force_transfer_call(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    fn mt_force_batch_transfer_call(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}

#[ext_contract(ext_mt_force_withdraw)]
pub trait MultiTokenForcedWithdrawer: MultiTokenWithdrawer + AccessControllable {
    fn mt_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
}
