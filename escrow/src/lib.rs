use defuse_nep245::receiver::MultiTokenReceiver;
use near_contract_standards::{
    fungible_token::receiver::FungibleTokenReceiver,
    non_fungible_token::core::NonFungibleTokenReceiver,
};
use near_sdk::{PromiseOrValue, ext_contract, serde_json};

mod action;
#[cfg(feature = "contract")]
mod contract;
mod intent;

#[ext_contract(ext_escrow)]
pub trait Escrow: FungibleTokenReceiver + NonFungibleTokenReceiver + MultiTokenReceiver {
    // TODO
    fn execute(&mut self) -> PromiseOrValue<serde_json::Value>;
}
