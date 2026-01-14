#![allow(clippy::too_many_arguments)]

use defuse_nep245::{MultiTokenCore, TokenId, receiver::MultiTokenReceiver};
use near_plugins::AccessControllable;
use near_sdk::{AccountId, Gas, PromiseOrValue, ext_contract, json_types::U128, near};

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

/// Action to perform when self-transferring (receiver_id == current_account_id).
/// This allows generic contracts (like escrow) to trigger withdrawals using
/// only standard NEP-245 methods.
#[must_use]
#[near(serializers = [json])]
#[serde(tag = "action", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum SelfAction {
    /// Withdraw tokens to an external contract via mt_batch_transfer
    Withdraw(WithdrawAction),
}

#[must_use]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct WithdrawAction {
    /// The external token contract to call mt_batch_transfer on
    pub token: AccountId,
    /// Final receiver of the tokens on the external contract
    pub receiver_id: AccountId,
    /// Optional memo for the external transfer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Optional message for mt_batch_transfer_call on the external contract
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// Optional minimum gas for the withdrawal
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}
