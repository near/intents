#![doc = include_str!("../README.md")]

#[cfg(feature = "near-kit")]
pub mod client;
#[cfg(feature = "near-contract")]
pub mod contract;
mod error;
mod events;
mod state;

pub use self::{error::*, events::*, state::*};

#[cfg(any(feature = "near-kit", feature = "near-contract"))]
pub use ::{
    defuse_borsh_utils::{AsWrap, Remainder},
    defuse_serde_utils::hex::AsHex,
};
