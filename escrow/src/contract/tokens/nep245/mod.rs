use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128};

use crate::contract::Contract;

impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        todo!()
    }
}
