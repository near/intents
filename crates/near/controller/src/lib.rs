#[cfg(feature = "near-kit")]
pub mod client;

#[cfg(feature = "contract")]
mod contract;
#[cfg(feature = "contract")]
pub use self::contract::*;

use borsh::BorshSerialize;
use near_gas::NearGas as Gas;

#[derive(BorshSerialize)]
pub struct UpgradeArgs<'a> {
    pub code: &'a [u8],
    pub state_migration_gas: Option<Gas>,
}
