mod tokens;

use near_sdk::{PromiseOrValue, near, serde_json};

use crate::Escrow;

#[near(
    contract_state,
    // TODO
)]
pub struct Contract {}

impl Escrow for Contract {
    fn execute(&mut self) -> PromiseOrValue<serde_json::Value> {
        todo!()
    }
}
