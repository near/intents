use near_sdk::{
    GlobalContractId,
    borsh::BorshSerialize,
    state_init::{StateInit, StateInitV1},
};

use defuse_wallet_state::State;

//TODO: move to separate crate?
pub trait StateExt {
    fn state_init(&self, code: GlobalContractId) -> StateInit;
}

impl<PubKey: BorshSerialize> StateExt for State<PubKey> {
    fn state_init(&self, code: GlobalContractId) -> StateInit {
        StateInit::V1(StateInitV1 {
            code,
            data: self.as_storage(),
        })
    }
}
