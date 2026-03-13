use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::Deadline;

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
    pub deadline: Deadline,
    pub nonce: T,
}

impl<T> ExpirableNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub const fn new(deadline: Deadline, nonce: T) -> Self {
        Self { deadline, nonce }
    }

    #[inline]
    pub fn has_expired(&self) -> bool {
        self.deadline.has_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{Days, Utc};
    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    #[rstest]
    fn expirable_test(random_bytes: Vec<u8>) {
        let current_timestamp = Utc::now();
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 24] = u.arbitrary().unwrap();

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
