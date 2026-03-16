use near_sdk::{AccountId, PromiseOrValue, ext_contract, json_types::U128};

use super::TokenId;

#[ext_contract(ext_mt_receiver)]
pub trait MultiTokenReceiver {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}
