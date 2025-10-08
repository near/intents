use impl_tools::autoimpl;
use near_sdk::{near, store::LookupSet};

use crate::contract::{
    ContractStorage, MigrateStorageWithPrefix, Prefix,
    accounts::Accounts,
    state::{ContractState, ContractStateV0},
};

#[derive(Debug)]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
#[near(serializers = [borsh])]
pub struct ContractStorageV0 {
    accounts: Accounts,

    state: ContractStateV0,

    relayer_keys: LookupSet<near_sdk::PublicKey>,
}

impl From<ContractStorageV0> for ContractStorage {
    fn from(
        ContractStorageV0 {
            accounts,
            state,
            relayer_keys,
        }: ContractStorageV0,
    ) -> Self {
        Self {
            accounts,
            state: ContractState::migrate(state, Prefix::State),
            relayer_keys,
        }
    }
}
