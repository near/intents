use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::DateTime;

/// Expirable nonces contain deadline which is 8 bytes of timestamp in nanoseconds
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct ExpirableNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    #[borsh(
        serialize_with = "As::<TimestampNanoSeconds>::serialize",
        deserialize_with = "As::<TimestampNanoSeconds>::deserialize"
    )]
    pub deadline: DateTime,
    pub nonce: T,
}

impl<T> ExpirableNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub const fn new(deadline: DateTime, nonce: T) -> Self {
        Self { deadline, nonce }
    }

    #[inline]
    pub fn has_expired(&self) -> bool {
        self.deadline.has_passed()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 24] = u.arbitrary().unwrap();

        let expired = ExpirableNonce::new(DateTime::now() - Duration::from_hours(24), nonce);
        assert!(expired.has_expired());

        let not_expired = ExpirableNonce::new(DateTime::timeout(Duration::from_hours(24)), nonce);
        assert!(!not_expired.has_expired());
    }
}
