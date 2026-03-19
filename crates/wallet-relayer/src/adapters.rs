use near_kit::{
    CryptoHash, DeterministicAccountStateInit, DeterministicAccountStateInitV1,
    GlobalContractIdentifier,
};
use near_sdk::{
    GlobalContractId,
    state_init::{StateInit, StateInitV1},
};

#[inline]
pub fn state_init(state_init: StateInit) -> DeterministicAccountStateInit {
    match state_init {
        StateInit::V1(StateInitV1 { code, data }) => {
            DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                code: global_contract_id(code),
                data,
            })
        }
    }
}

#[inline]
pub fn global_contract_id(global_contract_id: GlobalContractId) -> GlobalContractIdentifier {
    match global_contract_id {
        GlobalContractId::CodeHash(hash) => {
            GlobalContractIdentifier::CodeHash(CryptoHash::from_bytes(hash.into()))
        }
        GlobalContractId::AccountId(account_id) => GlobalContractIdentifier::AccountId(account_id),
    }
}
