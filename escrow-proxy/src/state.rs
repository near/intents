use std::collections::BTreeMap;

use crate::Error;
use near_sdk::{AccountId, GlobalContractId, borsh, near};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub owner_id: AccountId,
    pub oneshot_condvar_global_id: GlobalContractId,
    pub auth_contract: AccountId,
    pub notifier: AccountId,
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
