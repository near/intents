mod v0;
mod versioned;

pub use versioned::MaybeVersionedStateEntry;

use defuse_core::{ValidSalts, amounts::Amounts, fees::FeesConfig, token_id::TokenId};
use defuse_near_utils::NestPrefix;
use near_sdk::{
    AccountId, BorshStorageKey, IntoStorageKey, borsh::BorshSerialize, near, store::IterableMap,
};

pub type TokenBalances = Amounts<IterableMap<TokenId, u128>>;

#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct ContractState {
    pub total_supplies: TokenBalances,

    pub wnear_id: AccountId,

    pub fees: FeesConfig,

    pub salts: ValidSalts,
}

impl ContractState {
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
            salts: ValidSalts::new(prefix.as_slice().nest(Prefix::Salts)),
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum Prefix {
    TotalSupplies,
    Salts,
}
