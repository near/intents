mod borsh;
mod domain;
#[cfg(feature = "ed25519")]
pub mod ed25519;
mod hash;
pub mod no_sign;

#[cfg(feature = "webauthn")]
pub mod webauthn;

use std::time::Duration;

use defuse_borsh_utils::{As, DurationSeconds as BorshDurationSeconds};
use defuse_time::{Timestamp, borsh::TimestampSeconds};
use near_sdk::{AccountId, CryptoHash, env, near, serde_with::DurationSeconds};

use crate::Request;

pub use self::{borsh::*, domain::*, hash::*};

/// Signing standard, which defines the public key and how `signature` on
/// `msg` is verified.
pub trait SigningStandard<M> {
    /// Public key used by the signing standard.
    type PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool;
}
