use defuse_borsh_utils::As;
use defuse_time::{Timestamp, borsh::TimestampNanoSeconds};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// Expirable nonces contain deadline which is 8 bytes of timestamp in nanoseconds
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct ExpirableNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    #[borsh(
        serialize_with = "As::<TimestampNanoSeconds<i64>>::serialize",
        deserialize_with = "As::<TimestampNanoSeconds<i64>>::deserialize"
    )]
    pub deadline: Timestamp,
    pub nonce: T,
}

impl<T> ExpirableNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub const fn new(deadline: Timestamp, nonce: T) -> Self {
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

    use super::*;

    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 24] = u.arbitrary().unwrap();

        let expired = ExpirableNonce::new(Timestamp::now() - Duration::from_hours(24), nonce);
        assert!(expired.has_expired());

        let not_expired = ExpirableNonce::new(Timestamp::now() + Duration::from_hours(24), nonce);
        assert!(!not_expired.has_expired());
    }
}
