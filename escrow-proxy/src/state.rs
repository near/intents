use std::collections::BTreeMap;

use crate::Error;
use near_sdk::{AccountId, GlobalContractId, borsh, near};

/// Configuration for the escrow proxy contract.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    /// Account that owns this proxy instance.
    pub owner_id: AccountId,
    /// Global contract ID for oneshot condvar
    pub oneshot_condvar_global_id: GlobalContractId,
    /// Contract that will call `on_auth` on condvar instances.
    pub on_auth_caller: AccountId,
    /// Account to notify on authorization events.
    pub notifier_id: AccountId,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage(pub(crate) ProxyConfig);

impl ContractStorage {
    pub const STATE_KEY: &[u8] = b"";

    #[inline]
    pub const fn init(config: ProxyConfig) -> Self {
        Self(config)
    }

    pub fn init_state(config: ProxyConfig) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let storage = Self::init(config);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&storage).map_err(Error::Borsh)?,
        )]
        .into())
    }

    #[inline]
    pub const fn config(&self) -> &ProxyConfig {
        &self.0
    }
}
