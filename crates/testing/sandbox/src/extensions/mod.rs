use near_sdk::Gas;

// #[cfg(feature = "defuse")]
// pub mod defuse;
// #[cfg(feature = "escrow")]
// pub mod escrow;
// #[cfg(feature = "deployer")]
// pub mod global_deployer;
#[cfg(feature = "outlayer")]
pub mod outlayer_app;
#[cfg(feature = "poa")]
pub mod poa;
#[cfg(feature = "wallet")]
pub mod wallet;

pub const DEFAULT_GAS: Gas = Gas::from_tgas(300);
