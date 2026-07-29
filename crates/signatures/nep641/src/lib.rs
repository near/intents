#[cfg(feature = "near-kit")]
pub mod client;
pub mod contract;
mod message;
#[cfg(feature = "resolver")]
pub mod resolver;

pub use self::message::*;
