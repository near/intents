use near_kit::{
    CryptoHash, DeterministicAccountStateInit, DeterministicAccountStateInitV1,
    GlobalContractIdentifier,
};
use near_sdk::{
    GlobalContractId,
    state_init::{StateInit, StateInitV1},
};

pub trait IntoStateInit {
    fn into_state_init(self) -> DeterministicAccountStateInit;
}

impl IntoStateInit for StateInit {
    #[inline]
    fn into_state_init(self) -> DeterministicAccountStateInit {
        match self {
            StateInit::V1(StateInitV1 { code, data }) => {
                DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                    code: match code {
                        GlobalContractId::CodeHash(hash) => {
                            GlobalContractIdentifier::CodeHash(CryptoHash::from_bytes(hash.into()))
                        }
                        GlobalContractId::AccountId(account) => {
                            GlobalContractIdentifier::AccountId(account)
                        }
                    },
                    data,
                })
            }
        }
    }
}
