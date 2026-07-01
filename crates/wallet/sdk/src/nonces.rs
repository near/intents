use rand::{Rng, SeedableRng};

/// Endless [`Iterator`] for generating non-sequential nonces semi-sequentially,
/// allowing for multiple concurrent clients while being optimized for storage.
///
/// See [`crate::RequestMessage::nonce`].
#[derive(Debug)]
pub struct ConcurrentNonces<R> {
    next: u32,
    rng: R,
}

impl<R> ConcurrentNonces<R>
where
    R: Rng,
{
    const BIT_POS_MASK: u32 = (1 << u32::BITS.ilog2()) - 1;

    #[inline]
    pub const fn new(rng: R) -> Self {
        Self { next: 0, rng }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u32 {
        if self.next & Self::BIT_POS_MASK == 0 {
            self.next = self.rng.next_u32() & !Self::BIT_POS_MASK;
        }
        let n = self.next;
        self.next = self.next.wrapping_add(1);
        n
    }

    #[must_use]
    #[inline]
    pub fn fork(&mut self) -> Self
    where
        R: SeedableRng,
    {
        Self::new(self.rng.fork())
    }
}

impl<R> Iterator for ConcurrentNonces<R>
where
    R: Rng,
{
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_wallet_core::{State, Timestamp};
    use rand::rng;

    use super::*;

    #[test]
    fn zba() {
        const ZBA_TIMEOUT: Duration = Duration::from_mins(15); // 15m
        const MAX_SIZE: usize = 770 - 100 - 40; // TODO: -64?

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
