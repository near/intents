use defuse_bitmap::{BitMap256, U248, U256};
use defuse_map_utils::{IterableMap, Map};
use near_sdk::near;

pub type Nonce = U256;

// Prefix to identify expirable nonces:
// (first 4 bytes of sha256("expirable_nonce"))
const EXPIRABLE_NONCE_PREFIX: [u8; 4] = [0xdd, 0x50, 0xbc, 0x7c];

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
        self.0.remove(n)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Nonce> + '_
    where
        T: IterableMap,
    {
        self.0.as_iter()
    }
}

// To distinguish between legacy nonces and expirable nonces
// we use a specific prefix EXPIRABLE_NONCE_PREFIX. Expirable nonces
// have the following structure: [word_position, bit_position].
// Where word_position = [ EXPIRABLE_NONCE_PREFIX , <8 bytes timestamp in ms>, <20 random bytes> ]
pub struct ExpirableNonce {
    pub prefix: [u8; 4],
    pub timestamp: u64,
    pub data: [u8; 20],
}

impl From<Nonce> for ExpirableNonce {
    fn from(n: Nonce) -> Self {
        // It's safe to unwrap here because we know the entire slice is exactly 32 bytes long
        let prefix = n[0..4].try_into().unwrap();

        let timestamp_bytes = n[4..12].try_into().unwrap();
        let timestamp = u64::from_be_bytes(timestamp_bytes);

        let data = n[12..32].try_into().unwrap();

        ExpirableNonce {
            prefix,
            timestamp,
            data,
        }
    }
}

impl From<ExpirableNonce> for Nonce {
    fn from(n: ExpirableNonce) -> Self {
        let mut result = [0u8; 32];
        result[0..4].copy_from_slice(&n.prefix);
        result[4..12].copy_from_slice(&n.timestamp.to_be_bytes());
        result[12..32].copy_from_slice(&n.data);
        result
    }
}

impl ExpirableNonce {
    #[inline]
    pub fn is_expired(&self, current_timestamp: u64) -> bool {
        match self.prefix {
            EXPIRABLE_NONCE_PREFIX => self.timestamp < current_timestamp,
            _ => false, // Legacy nonces never expire
        }
    }

    pub fn pack_expirable(timestamp: u64, data: &[u8]) -> Result<Self, &'static str> {
        if data.len() != 20 {
            return Err("Invalid data length");
        }

        Ok(ExpirableNonce {
            prefix: EXPIRABLE_NONCE_PREFIX,
            timestamp,
            data: data.try_into().unwrap(),
        })
    }
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
        let current_timestamp = Utc::now().timestamp_millis() as u64;
        let nonce: U256 = u.arbitrary().unwrap();
        let nonexpirable = ExpirableNonce::from(nonce);

        assert!(!nonexpirable.is_expired(current_timestamp));
    }

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let current_timestamp = Utc::now().timestamp_millis() as u64;
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let seed: [u8; 20] = u.arbitrary().unwrap();

        let expired = ExpirableNonce::pack_expirable(current_timestamp - 1000, &seed).unwrap();
        assert!(expired.is_expired(current_timestamp));

        let not_expired = ExpirableNonce::pack_expirable(current_timestamp + 1000, &seed).unwrap();
        assert!(!not_expired.is_expired(current_timestamp));
    }
}
