pub use defuse_escrow_swap as contract;

use defuse_escrow_swap::{Params, Storage};

#[near_kit::contract]
pub trait Escrow {
    #[call]
    fn es_close(&mut self, params: Params) -> bool;

    #[call]
    fn es_lost_found(&mut self, params: Params) -> bool;

    fn es_view(&self) -> Storage;
}
