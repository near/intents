use defuse_bitmap::{BitMap256, U248, U256};
use defuse_map_utils::{IterableMap, Map};
use near_sdk::near;

pub type Nonce = U256;

// NOTE:
// Expirable nonce structure: [word_position, bit_position]
// Where word_position = [ EXPIRABLE_NONCE_PREFIX , <8 bytes timestamp in ms>, <22 random bytes> ]
const EXPIRABLE_NONCE_PREFIX: u8 = 0xFF;

/// See [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema)
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default)]
pub struct Nonces<T: Map<K = U248, V = U256>>(BitMap256<T>);

impl<T> Nonces<T>
where
    T: Map<K = U248, V = U256>,
{
    #[inline]
    pub const fn new(bitmap: T) -> Self {
        Self(BitMap256::new(bitmap))
    }

    #[inline]
    pub fn is_used(&self, n: Nonce) -> bool {
        self.0.get_bit(n)
    }

    #[inline]
    pub fn commit(&mut self, n: Nonce) -> bool {
        !self.0.set_bit(n)
    }

    #[inline]
    pub fn clear_expired(&mut self, n: Nonce) -> bool {
        !self.0.remove(n)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Nonce> + '_
    where
        T: IterableMap,
    {
        self.0.as_iter()
    }
}

#[inline]
pub fn is_nonce_expired(n: Nonce, current_timestamp: u64) -> bool {
    match n[0] {
        EXPIRABLE_NONCE_PREFIX => {
            // It's safe to unwrap here because we know the entire slice is exactly 32 bytes long
            let timestamp_bytes = n[1..9].try_into().unwrap();
            let timestamp = u64::from_be_bytes(timestamp_bytes);

            timestamp < current_timestamp
        }
        _ => false, // Legacy nonces never expire
    }
}

pub fn pack_expirable_nonce(timestamp: u64, seed: &[u8]) -> U256 {
    let mut result = [0u8; 32];

    result[0] = EXPIRABLE_NONCE_PREFIX;
    result[1..9].copy_from_slice(&timestamp.to_be_bytes());
    result[9..31].copy_from_slice(&seed);
    result[31] = 1;

    U256::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Unstructured;
    use chrono::Utc;
    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    #[rstest]
    fn nonexpirable_test(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonexpirable: U256 = u.arbitrary().unwrap();
        let current_timestamp = Utc::now().timestamp_millis() as u64;

        assert!(!is_nonce_expired(nonexpirable, current_timestamp));
    }

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let current_timestamp = Utc::now().timestamp_millis() as u64;
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let seed: [u8; 22] = u.arbitrary().unwrap();

        let expired = pack_expirable_nonce(current_timestamp - 1000, &seed);

        assert!(is_nonce_expired(expired, current_timestamp));

        let not_expired = pack_expirable_nonce(current_timestamp + 1000, &seed);

        assert!(!is_nonce_expired(not_expired, current_timestamp));
    }
}
