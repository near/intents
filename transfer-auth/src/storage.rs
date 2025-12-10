use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, YieldId, borsh, near};

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
pub struct State {
    pub escrow_contract_id: AccountId,
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
    pub state_init: State,
    //TODO:rename to state
    pub fsm: StateMachine,
}

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub fn new(state: State) -> Self {
        Self {
            state_init: state,
            fsm: StateMachine::Idle,
        }
    }

    pub fn init_state(state: State) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let storage = Self::new(state);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&storage).map_err(Error::Borsh)?,
        )]
        .into())
    }
}
