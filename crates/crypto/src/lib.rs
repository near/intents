//! Internal cryptographic primitives used across the Intents ecosystem.
//!
//! This crate defines lightweight traits such as [`Payload`] and
//! [`SignedPayload`] that allow Intents to treat messages from different
//! signing standards uniformly. Implementations of these traits live in
//! companion crates like `tip191`, `erc191`, or `bip322` and are primarily
//! intended for internal use.

mod curve;
#[cfg(feature = "parse")]
mod error;
mod payload;
#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
mod signature;

#[cfg(feature = "parse")]
pub use self::error::ParseCurveError;

pub use self::{curve::*, payload::*};

#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
pub use self::signature::*;

#[cfg(all(
    any(feature = "ed25519", feature = "secp256k1", feature = "p256"),
    feature = "serde"
))]
pub mod serde;
