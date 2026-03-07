use core::{mem, time::Duration};

use defuse_bitmap::CompactBitMap;
use defuse_borsh_utils::adapters::{As, DurationSeconds as BorshDurationSeconds, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::{near, serde_with::DurationSeconds};

use crate::{Error, Result};

// TODO: current now() % number of already submitted nonces
// nonces are unbounded: BitVec?
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nonces {
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

    old_nonces: CompactBitMap<u32>, // TODO: corresponds to previous epoch?
    nonces: CompactBitMap<u32>,     // corresponds to current epoch?

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

impl Nonces {
    #[inline]
    pub const fn new(timeout: Duration) -> Self {
        Self {
            last_cleaned_at: Deadline::MIN,
            old_nonces: CompactBitMap::new(),
            nonces: CompactBitMap::new(),
            timeout,
        }
    }

    #[inline]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[inline]
    pub const fn last_cleaned_at(&self) -> Deadline {
        self.last_cleaned_at
    }
}

impl Nonces {
    pub fn commit(&mut self, nonce: u32, created_at: Deadline, timeout: Duration) -> Result<()> {
        if timeout != self.timeout {
            return Err(Error::InvalidTimeout);
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
                self.old_nonces = CompactBitMap::new();
            }
            // update last rotation time
            self.last_cleaned_at = now;
        }

        if !(last_valid_nonce_at <= created_at && created_at <= now) {
            return Err(Error::InvalidCreatedAt);
        }

        if self.old_nonces.get_bit(nonce) || self.nonces.set_bit(nonce) {
            return Err(Error::AlreadyExecuted);
        }

        Ok(())
    }
}
