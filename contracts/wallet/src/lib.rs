#![doc = include_str!("../README.md")]

#[cfg(feature = "near-contract")]
pub mod contract;
mod error;
pub mod events;
mod message;
mod nonces;
#[cfg(feature = "readable-message")]
pub mod readable_message;
mod request;
mod schema;
#[cfg(feature = "serde")]
mod serde_adapters;
mod state;
pub use self::{error::*, message::*, nonces::*, request::*, schema::*, state::*};

pub use defuse_nep641 as offchain;
pub use defuse_time::Timestamp;
pub use near_account_id::{AccountId, AccountIdRef};

// re-export for `wallet!` macro
#[cfg(feature = "near-contract")]
#[doc(hidden)]
pub use near_sdk;
