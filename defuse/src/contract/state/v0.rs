use defuse_core::{SaltRegistry, fees::FeesConfig};
use defuse_near_utils::NestPrefix;
use near_sdk::{AccountId, IntoStorageKey, near};

use crate::contract::{
    Prefix as ContractPrefix,
    state::{ContractState, Prefix as StatePrefix, TokenBalances},
};

#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct ContractStateV0 {
    pub total_supplies: TokenBalances,

    pub wnear_id: AccountId,

    pub fees: FeesConfig,
}

// TODO: move it
impl From<ContractStateV0> for ContractState {
    fn from(
        ContractStateV0 {
            total_supplies,
            wnear_id,
            fees,
        }: ContractStateV0,
    ) -> Self {
        Self {
            total_supplies,
            wnear_id,
            fees,
            salts: SaltRegistry::new(
                ContractPrefix::State
                    .into_storage_key()
                    .nest(StatePrefix::Salts),
            ),
        }
    }
}
