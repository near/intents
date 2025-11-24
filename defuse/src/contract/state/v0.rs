use defuse_core::{SaltRegistry, fees::FeesConfig};
use defuse_near_utils::NestPrefix;
use near_sdk::{AccountId, IntoStorageKey, near};

use crate::contract::{
    MigrateStorageWithPrefix,
    state::{ContractState, Prefix, TokenBalances},
};

#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct ContractStateV0 {
    pub total_supplies: TokenBalances,

    pub wnear_id: AccountId,

    pub fees: FeesConfig,
}

impl MigrateStorageWithPrefix<ContractStateV0> for ContractState {
    fn migrate<S>(
        ContractStateV0 {
            total_supplies,
            wnear_id,
            fees,
        }: ContractStateV0,
        prefix: S,
    ) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            total_supplies,
            wnear_id,
            fees,
            salts: SaltRegistry::new(prefix.into_storage_key().nest(Prefix::Salts)),
        }
    }
}

/// Legacy implementation of [`ContractStorageV0`]
#[cfg(test)]
pub(super) mod tests {

    use super::*;
    use near_sdk::{AccountId, store::IterableMap};

    impl ContractStateV0 {
        #[inline]
        pub fn new<S>(prefix: S, wnear_id: AccountId, fees: FeesConfig) -> Self
        where
            S: IntoStorageKey,
        {
            let prefix = prefix.into_storage_key();
            Self {
                total_supplies: TokenBalances::new(IterableMap::new(
                    prefix.as_slice().nest(Prefix::TotalSupplies),
                )),
                wnear_id,
                fees,
            }
        }
    }
}
