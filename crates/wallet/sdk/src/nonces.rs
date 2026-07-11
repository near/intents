use rand::Rng;

#[cfg(feature = "tracing")]
use tracing::{Level, instrument, trace};

/// Semi-sequential nonce generation algorithm optimized for storage usage
/// in case of multiple concurrent **non-coordinated** clients sign messages
/// for the same wallet contract instance. This algorithm withstands signers
/// restarts and allows more signers to join silently while keeping collision
/// probability relatively low.
///
/// Unfortunately, [`.next()`](Self::next) needs a mutable reference and can't
/// achieve the same using only atomics because of the partial CAS operation
/// required when randomizing high 27 bits. See [`crate::RequestMessage::nonce`].
///
/// As a result, `ConcurrentNonces` doesn't implement [`Clone`](Clone). Clonable
/// same-account signers are recommended to wrap it in `Arc<Mutex<_>>` instead of
/// re-creating it every time, so that at least signers in the same process draw
/// nonces from a single shared pool.
#[derive(Debug)]
pub struct ConcurrentNonces<R> {
    next: u32,
    rng: R,
}

impl<R> ConcurrentNonces<R>
where
    R: Rng,
{
    /// 000000000000000000000000.11111
    const BIT_POS_MASK: u32 = (1 << u32::BITS.ilog2()) - 1;

    #[inline]
    pub const fn new(rng: R) -> Self {
        Self { next: 0, rng }
    }

    /// Get the next semi-sequential nonce ready to use.
    #[allow(clippy::should_implement_trait)]
    #[cfg_attr(feature = "tracing", instrument(
        level = Level::TRACE, skip_all, ret(Display),
    ))]
    #[inline]
    pub fn next(&mut self) -> u32 {
        // check if we're at block boundary
        if self.next & Self::BIT_POS_MASK == 0 {
            #[cfg(feature = "tracing")]
            trace!(
                current = %self.next,
                "we're at block boundary, randomizing high 27 bits..."
            );

            // randomize high 27 bits
            self.next = self.rng.next_u32() & !Self::BIT_POS_MASK;
        }
        let n = self.next;
        self.next = self.next.wrapping_add(1);
        n
    }
}

impl<R> Iterator for ConcurrentNonces<R>
where
    R: Rng,
{
    type Item = u32;

    /// Always returns the next nonce ready to use.
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_wallet::{State, Timestamp};
    use rand::rng;

    use super::*;

    #[test]
    fn zba() {
        const ZBA_TIMEOUT: Duration = Duration::from_mins(15); // 15m
        const MAX_SIZE: usize = 770 - 100 - 40 - 64;

        const PUBLIC_KEY: [u8; 64] = [0u8; 64];

        let mut ns = ConcurrentNonces::new(rng());

        for _ in 0..1000 {
            let mut state = State::new(PUBLIC_KEY).timeout(ZBA_TIMEOUT);
            let created_at = Timestamp::now() - Duration::from_mins(1);

            for n in ns
                .by_ref()
                // 1 tx/s
                .take(ZBA_TIMEOUT.as_secs().try_into().unwrap())
            {
                state
                    .nonces
                    .commit(n, created_at, ZBA_TIMEOUT)
                    .expect("rand collision");
            }

            let serialized_len = borsh::to_vec(&state).unwrap().len();
            assert!(
                serialized_len <= MAX_SIZE,
                "state would not fit into ZBA limits: {serialized_len} bytes",
            );
        }
    }
}
