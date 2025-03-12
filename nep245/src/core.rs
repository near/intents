use near_sdk::{AccountId, PromiseOrValue, ext_contract, json_types::U128};

use super::{Token, TokenId, resolver::MultiTokenResolver};

#[ext_contract(ext_mt_core)]
pub trait MultiTokenCore: MultiTokenResolver {
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    );

    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    );

    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>>;

    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128;

    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128>;

    fn mt_supply(&self, token_id: TokenId) -> Option<U128>;

    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>>;
}
