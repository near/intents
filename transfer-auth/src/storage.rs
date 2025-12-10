use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, GlobalContractId, YieldId, borsh, near};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum StateMachine {
    Idle,
    WaitingForAuthorization(YieldId),
    Authorized,
    Done,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInit {
    pub escrow_contract_id: GlobalContractId,
    #[cfg(feature = "auth-call")]
    pub auth_contract: AccountId,
    pub on_auth_signer: AccountId,
    pub authorizee: AccountId,
    //TODO: fix
    // #[serde_as(as = "Hex")]
    pub msg_hash: [u8; 32],
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage{
    #[serde(flatten)]
    pub state_init: StateInit,
    pub state: StateMachine,
}

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub fn new(state_init: StateInit) -> Self {
        Self {
            state_init,
            state: StateMachine::Idle,
        }
    }

    pub fn init_state(state_init: StateInit) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let storage = Self::new(state_init);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&storage).map_err(Error::Borsh)?,
        )]
        .into())
    }
}
