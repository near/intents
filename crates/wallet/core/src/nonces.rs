use core::{mem, time::Duration};
use std::collections::BTreeMap;

use defuse_bitmap::BitMap;
use defuse_time::Timestamp;

#[cfg(feature = "borsh")]
use ::{
    defuse_borsh_utils::{As, DurationSeconds},
    defuse_time::borsh::TimestampNanoSeconds,
};

/// Recommended timeout for production use: `1 hour`.
///
/// This allows messages to survive relayer/chain congestions
/// with reasonable storage usage under typical load.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_hours(1);

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
/// Dual-timeout window nonces
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nonces {
    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<DurationSeconds<u32>>::serialize",
            deserialize_with = "As::<DurationSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<DurationSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<DurationSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<DurationSeconds<u32>>::serialize",
            deserialize_with = "As::<DurationSeconds<u32>>::deserialize",
        )
    )]
    /// Fixed timeout, i.e. maximum validity timespan for each nonce.
    timeout: Duration,

    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampNanoSeconds<u64>>::add_definitions_recursively",
                declaration = "As::<TimestampNanoSeconds<u64>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
        )
    )]
    /// The last timestamp when nonces were rotated
    last_cleaned_at: Timestamp,

    /// Previous nonces, i.e. within `[now - 2*timeout, now - timeout)`
    old: BitMap<BTreeMap<u32, u32>>,
    /// Current nonces, i.e. within `[now - timeout, now]`
    current: BitMap<BTreeMap<u32, u32>>,
}

impl Default for Nonces {
    fn default() -> Self {
        Self::new(DEFAULT_TIMEOUT)
    }
}

impl Nonces {
    #[inline]
    pub const fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_cleaned_at: Timestamp::UNIX_EPOCH,
            old: BitMap::new(BTreeMap::new()),
            current: BitMap::new(BTreeMap::new()),
        }
    }

    #[cfg(feature = "std")]
    /// Commit the nonce which was created at given time for given timeout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::time::Duration;
    /// # use defuse_wallet_core::{DEFAULT_TIMEOUT, Nonces, Timestamp};
    /// let mut nonces = Nonces::new(DEFAULT_TIMEOUT);
    /// let created_at = Timestamp::now() - Duration::from_mins(1);
    ///
    /// nonces.commit(
    ///     123,
    ///     Timestamp::now() - Duration::from_mins(1),
    ///     DEFAULT_TIMEOUT
    /// ).unwrap(); // ok
    ///
    /// nonces.commit(
    ///     123,
    ///     Timestamp::now() - Duration::from_mins(1),
    ///     DEFAULT_TIMEOUT
    /// ).unwrap_err(); // already used
    /// ```
    pub fn commit(
        &mut self,
        nonce: u32,
        created_at: Timestamp,
        timeout: Duration,
    ) -> Result<(), NonceError> {
        self.check_cleanup();

        let now = Timestamp::now();
        // check that `created_at` is in `[now - min(self.timeout, msg.timeout), now]`
        if !(now - self.timeout.min(timeout) <= created_at && created_at <= now) {
            return Err(NonceError::ExpiredOrFuture);
        }

        if self.old.get_bit(nonce) || self.current.set_bit(nonce) {
            return Err(NonceError::AlreadyUsed);
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    /// Rotate and cleanup if it's time
    pub fn check_cleanup(&mut self) {
        let now = Timestamp::now();
        let last_valid_nonce_at = now - self.timeout;

        // check if it's time to rotate
        if self.last_cleaned_at < last_valid_nonce_at {
            // rotate current -> old
            self.old = mem::take(&mut self.current);
            // check if `2 * timeout` has passed since last rotation
            if self.last_cleaned_at < last_valid_nonce_at - self.timeout {
                // cleanup old nonces
                self.old = BitMap::new(BTreeMap::new());
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
    pub const fn last_cleaned_at(&self) -> Timestamp {
        self.last_cleaned_at
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NonceError {
    #[error("has already been used")]
    AlreadyUsed,
    #[error("expired or from the future")]
    ExpiredOrFuture,
}
