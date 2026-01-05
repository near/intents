use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, GlobalContractId, YieldId, borsh, near};
use serde_with::{hex::Hex, serde_as};

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
    #[serde_as(as = "Hex")]
    pub msg_hash: [u8; 32],
}

/// The actual state data containing initialization params and state machine
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    #[serde(flatten)]
    pub state_init: StateInit,
    pub state: StateMachine,
}

impl State {
    #[inline]
    pub const fn new(state_init: StateInit) -> Self {
        Self {
            state_init,
            state: StateMachine::Idle,
        }
    }
}

/// Contract storage wrapper - None means cleanup is in progress
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage(
    /// If `None`, authorization completed and contract is being deleted
    pub(crate) Option<State>,
);

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub fn init(state_init: StateInit) -> Self {
        Self(Some(State::new(state_init)))
    }

    pub fn init_state(state_init: StateInit) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let storage = Self::init(state_init);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&storage).map_err(Error::Borsh)?,
        )]
        .into())
    }
}
