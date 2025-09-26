mod tokens;

use defuse_token_id::TokenId;
use near_sdk::{PromiseOrValue, near, serde_json};

use crate::Escrow;

#[near(
    contract_state,
    // TODO
)]
pub struct Contract {
    maker_asset: TokenId,
    maker_amount: u128,
    taker_asset: TokenId,
    taker_amount: u128,
}

impl Escrow for Contract {
    fn execute(&mut self) -> PromiseOrValue<serde_json::Value> {
        todo!()
    }
}
