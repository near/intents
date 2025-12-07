use defuse_auth_call::AuthCallee;
use near_sdk::{
    ext_contract, near
};

mod error;
#[cfg(feature = "contract")]
mod contract;
pub mod storage;

use storage::{ContractStorage, State};


#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &ContractStorage;
}
