use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, GlobalContractId, YieldId, borsh, near};
use serde_with::{hex::Hex, serde_as};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Status {
    Idle,
    WaitingForNotification(YieldId),
    Notified,
    Done,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub escrow_contract_id: GlobalContractId,
    #[cfg(feature = "auth-call")]
    pub auth_contract: AccountId,
    pub notifier_id: AccountId,
    pub authorizee: AccountId,
    #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

/// The actual state data containing initialization params and state machine
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    #[serde(flatten)]
    pub config: Config,
    pub state: Status,
}

impl State {
    #[inline]
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            state: Status::Idle,
        }
    }
}

/// Contract storage wrapper - None means cleanup is in progress
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage(
    /// If `None`, notification completed and contract is being deleted
    pub(crate) Option<State>,
);

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub const fn init(config: Config) -> Self {
        Self(Some(State::new(config)))
    }

    pub fn init_state(config: Config) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let storage = Self::init(config);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&storage).map_err(Error::Borsh)?,
        )]
        .into())
    }
}
