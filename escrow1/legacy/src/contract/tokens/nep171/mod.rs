use near_contract_standards::non_fungible_token::{TokenId, core::NonFungibleTokenReceiver};
use near_sdk::{AccountId, PromiseOrValue};

use crate::contract::Contract;

impl NonFungibleTokenReceiver for Contract {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        todo!()
    }
}
