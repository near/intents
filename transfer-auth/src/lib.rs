use defuse_auth_call::AuthCallee;
use near_sdk::{
    ext_contract, near
};

mod error;
#[cfg(feature = "contract")]
mod contract;
pub mod storage;

use storage::{ContractStorage, State};


//TODO: allow proxy to delete contract on demand (handle yielded proxy)
//TODO: implement auto-deletion when Finished state is reached
#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &ContractStorage;
}
