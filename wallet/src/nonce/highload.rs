use core::{mem, time::Duration};

use defuse_bitmap::CompactBitMap;
use defuse_borsh_utils::adapters::{As, DurationSeconds as BorshDurationSeconds, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::{near, serde_with::DurationSeconds};
use thiserror::Error as ThisError;

use crate::Nonces;

type Nonce = u32;
type BitMap = CompactBitMap<Nonce>;

// TODO: current now() % number of already submitted nonces
// nonces are unbounded: BitVec?
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HighloadNonces {
    // TODO: can we make it deterministic?
    #[cfg_attr(
        feature = "abi",
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
        )
    )]
    last_cleaned_at: Deadline,

    old_nonces: BitMap, // corresponds to previous epoch?
    nonces: BitMap,     // corresponds to current epoch?

    #[serde(rename = "timeout_secs")]
    #[serde_as(as = "DurationSeconds")]
    #[cfg_attr(
        feature = "abi",
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<BorshDurationSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<BorshDurationSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
        )
    )]
    timeout: Duration,
}

impl Nonces for HighloadNonces {
    type Nonce = TimeoutNonce;
    type Error = TimeoutNonceError;

    fn commit(&mut self, nonce: Self::Nonce) -> Result<(), Self::Error> {
        if nonce.timeout != self.timeout {
            return Err(TimeoutNonceError::InvalidTimeout); // TODO
        }

        let now = Deadline::now();
        let last_valid_nonce_at = now - self.timeout;

        // check if it's time to rotate
        if self.last_cleaned_at < last_valid_nonce_at {
            // rotate current -> old
            self.old_nonces = mem::take(&mut self.nonces);
            // check if `2 * timeout` has passed since last rotation
            if self.last_cleaned_at < last_valid_nonce_at - self.timeout {
                // cleanup old nonces
                self.old_nonces = BitMap::new();
            }
            // update last rotation time
            self.last_cleaned_at = now;
        }

        if !(last_valid_nonce_at <= nonce.created_at && nonce.created_at <= now) {
            return Err(TimeoutNonceError::ExpiredOrFromFuture);
        }

        if self.old_nonces.get_bit(nonce.nonce) || self.nonces.set_bit(nonce.nonce) {
            return Err(TimeoutNonceError::AlreadyExecuted);
        }

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeoutNonce {
    pub nonce: Nonce,
    #[cfg_attr(
        feature = "abi",
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
        )
    )]
    pub created_at: Deadline,
    #[cfg_attr(
        feature = "abi",
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<BorshDurationSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<BorshDurationSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
        )
    )]
    pub timeout: Duration,
}

#[derive(Debug, ThisError, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimeoutNonceError {
    #[error("expired or from the future")]
    ExpiredOrFromFuture,

    #[error("invalid timeout")]
    InvalidTimeout,

    #[error("already executed")]
    AlreadyExecuted,
}
