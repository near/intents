use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{AccountId, PromiseOrValue, json_types::U128};

use crate::contract::Contract;

impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        todo!()
    }
}
