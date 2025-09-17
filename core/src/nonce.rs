use defuse_bitmap::{BitMap256, U248, U256};
use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
use defuse_map_utils::{IterableMap, Map};
use hex_literal::hex;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near,
};

use crate::{Deadline, DefuseError, Result};

pub type Nonce = U256;

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
    pub fn commit(&mut self, n: Nonce) -> Result<()> {
        if ExpirableNonce::maybe_from(n).is_some_and(|expirable| expirable.has_expired()) {
            return Err(DefuseError::NonceExpired);
        }

        if self.0.set_bit(n) {
            return Err(DefuseError::NonceUsed);
        }

        Ok(())
    }

    #[inline]
    pub fn clear_expired(&mut self, n: Nonce) -> bool {
        if ExpirableNonce::maybe_from(n).is_some_and(|n| n.has_expired()) {
            let [prefix @ .., _] = n;
            return self.0.clear_by_prefix(prefix);
        }

        false
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Nonce> + '_
    where
        T: IterableMap,
    {
        self.0.as_iter()
    }
}

/// To distinguish between legacy nonces and expirable nonces
/// we use a specific prefix `EXPIRABLE_NONCE_PREFIX`. Expirable nonces
/// have the following structure: [`word_position`, `bit_position`].
/// Where `word_position` = [ `EXPIRABLE_NONCE_PREFIX` , <8 bytes timestamp in nanoseconds>, <19 random bytes> ]
/// and `bit_position` is the last (lowest) byte
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct ExpirableNonce {
    #[borsh(
        serialize_with = "As::<TimestampNanoSeconds>::serialize",
        deserialize_with = "As::<TimestampNanoSeconds>::deserialize"
    )]
    pub deadline: Deadline,
    pub nonce: [u8; 20],
}

impl From<ExpirableNonce> for Nonce {
    fn from(n: ExpirableNonce) -> Self {
        let mut result = [0u8; 32];

        borsh::to_writer(
            &mut result[..],
            &(ExpirableNonce::EXPIRABLE_NONCE_PREFIX, n),
        )
        .unwrap_or_else(|_| unreachable!());
        result
    }
}

impl ExpirableNonce {
    /// Prefix to identify expirable nonces:
    /// (first 4 bytes of `sha256("expirable_nonce"))`
    pub const EXPIRABLE_NONCE_PREFIX: [u8; 4] = hex!("dd50bc7c");

    pub const fn new(deadline: Deadline, nonce: [u8; 20]) -> Self {
        Self { deadline, nonce }
    }

    /// Checks prefix and parses the rest as expirable nonce
    /// If prefix doesn't match or nonce has invalid timestamp, returns None
    pub fn maybe_from(n: Nonce) -> Option<Self> {
        let mut bytes = n.strip_prefix(&Self::EXPIRABLE_NONCE_PREFIX)?;
        Self::deserialize_reader(&mut bytes).ok()
    }

    #[inline]
    pub fn has_expired(&self) -> bool {
        self.deadline.has_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Unstructured;
    use chrono::{Days, Utc};
    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    #[rstest]
    fn roundtrip_layout(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonce_bytes: [u8; 20] = u.arbitrary().unwrap();
        let now = Deadline::new(Utc::now());

        let exp = ExpirableNonce::new(now, nonce_bytes);
        let packed: Nonce = exp.clone().into();

        let unpacked = ExpirableNonce::maybe_from(packed).expect("prefix must match");
        assert_eq!(unpacked, exp);
    }

    #[rstest]
    fn nonexpirable_test(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonce: U256 = u.arbitrary().unwrap();
        let nonexpirable = ExpirableNonce::maybe_from(nonce);

        assert!(nonexpirable.is_none());
    }

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let current_timestamp = Utc::now();
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 20] = u.arbitrary().unwrap();

        let expired = ExpirableNonce::new(
            Deadline::new(current_timestamp.checked_sub_days(Days::new(1)).unwrap()),
            nonce,
        );
        assert!(expired.has_expired());

        let not_expired = ExpirableNonce::new(
            Deadline::new(current_timestamp.checked_add_days(Days::new(1)).unwrap()),
            nonce,
        );
        assert!(!not_expired.has_expired());
    }
}
