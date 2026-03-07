use rand_core::Rng;

/// Endless [`Iterator`] for generating non-sequential nonces semi-sequentially,
/// allowing for multiple concurrent clients while being optimized for storage.
///
/// See [`crate::RequestMessage`].
pub struct ConcurrentNonces<R> {
    next: u32,
    rng: R,
}

impl<R> ConcurrentNonces<R>
where
    R: Rng,
{
    const BIT_POS_MASK: u32 = 0b11111;

    pub const fn new(rng: R) -> Self {
        Self { next: 0, rng }
    }
}

impl<R> Iterator for ConcurrentNonces<R>
where
    R: Rng,
{
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next & Self::BIT_POS_MASK == 0 {
            self.next = self.rng.next_u32() & !Self::BIT_POS_MASK;
        }
        let n = self.next;
        self.next = self.next.wrapping_add(1);
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_bitmap::CompactBitMap;
    use near_sdk::borsh;
    use rand::rng;

    use super::*;

    #[test]
    fn zba() {
        const TIMEOUT: Duration = Duration::from_secs(60 * 15); // 15 mins
        const MAX_SIZE: usize = 235;

        let mut ns = ConcurrentNonces::new(rng()).peekable();

        for _ in 0..1000 {
            let mut nonces = CompactBitMap::<u32>::new();

            for n in ns
                .by_ref()
                // 1 tx/s
                .take(TIMEOUT.as_secs().try_into().unwrap())
            {
                assert!(!nonces.set_bit(n), "rand collision");
            }
            assert!(
                borsh::to_vec(&nonces).unwrap().len() <= MAX_SIZE,
                "state would not fit into ZBA limits"
            );
        }
    }
}
