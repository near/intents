use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, borsh, near};

// Re-export for backward compatibility
pub use crate::state_machine::StateMachine;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub solver_id: AccountId,
    pub escrow_contract_id: AccountId,
    pub auth_contract: AccountId,
    pub auth_callee: AccountId,
    pub querier: AccountId,
    // #[serde_as(as = "Hex")]
    pub msg_hash: [u8; 32],
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage {
    #[serde(flatten)]
    pub state_init: State,
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
        let state = Self::new(state);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&state).map_err(Error::Borsh)?,
        )]
        .into())
    }
}
