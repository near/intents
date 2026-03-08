#[cfg(feature = "concurrent")]
mod concurrent;
#[cfg(feature = "concurrent")]
pub use self::concurrent::*;

use core::{mem, time::Duration};
use std::collections::BTreeMap;

use defuse_bitmap::BitMap;
use defuse_borsh_utils::adapters::{As, DurationSeconds as BorshDurationSeconds, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::near;

use crate::{Error, Result};

/// Dual-timeout window nonces
#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nonces {
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
    /// Timeout, i.e. validity timespan for each nonce.
    timeout: Duration,

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
    /// The last timestamp when nonces were rotated
    last_cleaned_at: Deadline,

    /// Previous nonces, i.e. within `[now - 2*timeout, now - timeout)`
    old_nonces: BitMap<BTreeMap<u32, u32>>,
    /// Current nonces, i.e. within `[now - timeout, now]`
    nonces: BitMap<BTreeMap<u32, u32>>,
}

impl Nonces {
    #[inline]
    pub const fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_cleaned_at: Deadline::MIN,
            old_nonces: BitMap::new(BTreeMap::new()),
            nonces: BitMap::new(BTreeMap::new()),
        }
    }

    pub fn commit(&mut self, nonce: u32, created_at: Deadline, timeout: Duration) -> Result<()> {
        if timeout != self.timeout {
            return Err(Error::InvalidTimeout);
        }

        self.check_cleanup();

        let now = Deadline::now();
        if !(now - self.timeout <= created_at && created_at <= now) {
            return Err(Error::ExpiredOrFuture);
        }

        if self.old_nonces.get_bit(nonce) || self.nonces.set_bit(nonce) {
            return Err(Error::AlreadyExecuted);
        }

        Ok(())
    }

    /// Rotate and cleanup if it's time
    pub fn check_cleanup(&mut self) {
        let now = Deadline::now();
        let last_valid_nonce_at = now - self.timeout;

        // check if it's time to rotate
        if self.last_cleaned_at < last_valid_nonce_at {
            // rotate current -> old
            self.old_nonces = mem::take(&mut self.nonces);
            // check if `2 * timeout` has passed since last rotation
            if self.last_cleaned_at < last_valid_nonce_at - self.timeout {
                // cleanup old nonces
                self.old_nonces = BitMap::new(BTreeMap::new());
            }
            // update last rotation time
            self.last_cleaned_at = now;
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
