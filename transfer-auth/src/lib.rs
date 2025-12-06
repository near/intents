use defuse_auth_call::AuthCallee;
use near_sdk::{
    ext_contract, near
};

mod error;
#[cfg(feature = "contract")]
mod contract;
pub mod storage;

mod message;
use storage::{ContractStorage, State};
pub use message::AuthMessage;


#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &ContractStorage;
}
