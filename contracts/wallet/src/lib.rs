#![doc = include_str!("../README.md")]

#[cfg(feature = "near-contract")]
pub mod contract;
mod error;
pub mod events;
mod message;
mod nonces;
mod request;
mod schema;
mod state;
pub use self::{error::*, message::*, nonces::*, request::*, schema::*, state::*};

pub use defuse_time::Timestamp;
pub use near_account_id::{AccountId, AccountIdRef};

// re-export for `wallet!` macro
#[doc(hidden)]
#[cfg(feature = "near-contract")]
pub use near_sdk;
