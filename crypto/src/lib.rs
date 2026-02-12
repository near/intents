//! Internal cryptographic primitives used across the Intents ecosystem.
//!
//! This crate defines lightweight traits such as [`Payload`] and
//! [`SignedPayload`] that allow Intents to treat messages from different
//! signing standards uniformly. Implementations of these traits live in
//! companion crates like `tip191`, `erc191`, or `bip322` and are primarily
//! intended for internal use.

mod curve;
mod parse;
mod payload;
#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
mod public_key;
#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
mod signature;

pub use self::{curve::*, parse::ParseCurveError, payload::*};

#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
pub use self::{public_key::*, signature::*};

#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]
pub mod serde;
