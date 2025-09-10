//! Internal cryptographic primitives used across the Intents ecosystem.
//!
//! This crate defines lightweight traits such as [`Payload`] and
//! [`SignedPayload`] that allow Intents to treat messages from different
//! signing standards uniformly. Implementations of these traits live in
//! companion crates like `tip191`, `erc191`, or `bip322` and are primarily
//! intended for internal use.

mod curve;
mod payload;
mod public_key;
mod signature;

pub use self::{curve::*, payload::*, public_key::*, signature::*};

#[cfg(feature = "serde")]
pub mod serde;
