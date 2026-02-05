use std::collections::BTreeMap;

use crate::error::Error;
use near_sdk::{AccountId, YieldId, borsh, near};
use serde_with::{hex::Hex, serde_as};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Status {
    Idle,
    WaitingForNotification(YieldId),
    Notified,
    Done,
}

/// Configuration for the oneshot condition variable contract.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Account ID of the contract that is allowed to call `on_auth`.
    #[cfg(feature = "auth-call")]
    pub on_auth_caller: AccountId,
    /// Account ID that is permitted to call [`cv_notify_one`] on oneshot condvar
    pub notifier_id: AccountId,
    /// Account ID that becomes authorized upon successful notification.
    pub authorizee: AccountId,
    /// Unique salt used for deterministic account derivation.
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
