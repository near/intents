pub use defuse_escrow_swap as contract;

use defuse_escrow_swap::{Params, Storage};
use near_sdk::serde::{Deserialize, Serialize};

#[near_kit::contract]
pub trait Escrow {
    #[call]
    fn es_close(&mut self, args: EsArgs) -> bool;

    #[call]
    fn es_lost_found(&mut self, args: EsArgs) -> bool;

    fn es_view(&self) -> Storage;
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct EsArgs {
    pub params: Params,
}
