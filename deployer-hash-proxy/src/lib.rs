#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::collections::BTreeMap;

use defuse_global_deployer::OwnerProxyState;
use near_sdk::{
    Promise, borsh, ext_contract, near,
    serde_with::{hex::Hex, serde_as},
};

#[ext_contract(ext_deployer_hash_proxy)]
pub trait DeployerProxyHash {
    fn hp_approve(&mut self);

    fn hp_exec(
        &mut self,
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise;
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "global-deployer"))]
pub enum Event {
    #[event_version("1.0.0")]
    Approved {
        #[serde_as(as = "Hex")]
        old_hash: [u8; 32],
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Exec {
        #[serde_as(as = "Hex")]
        new_hash: [u8; 32],
    },
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    #[serde(flatten)]
    pub proxy: OwnerProxyState,
}

impl From<OwnerProxyState> for State {
    fn from(proxy: OwnerProxyState) -> Self {
        Self { proxy }
    }
}

impl State {
    pub const STATE_KEY: &[u8] = b"";

    pub const fn new(proxy: OwnerProxyState) -> Self {
        Self { proxy }
    }

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtraParams {
    pub approved: bool,
}

impl ExtraParams {
    pub const STORAGE_KEY: &[u8] = b"e";

    pub fn read() -> Self {
        near_sdk::env::storage_read(Self::STORAGE_KEY)
            .map(|bytes| borsh::from_slice(&bytes).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn write(&self) {
        near_sdk::env::storage_write(
            Self::STORAGE_KEY,
            &borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        );
    }
}
