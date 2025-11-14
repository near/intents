use hex_literal::hex;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    DefuseError, Nonce, Result,
    nonce::{expirable::ExpirableNonce, salted::SaltedNonce},
};

/// To distinguish between legacy nonces and versioned nonces
/// we use a specific prefix individual for each version.
/// Serialized versioned nonce contains:
///     `VERSIONED_MAGIC_PREFIX (4 bytes) || VERSION (1 byte) || NONCE_BYTES (27 bytes)`
/// Currently supported versions:
///     - V1: `SALT (4 bytes) || DEADLINE (8 bytes) || NONCE (15 random bytes)`
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub enum VersionedNonce {
    V1(SaltedNonce<ExpirableNonce<[u8; 15]>>),
}

// NOTE: Legacy nonces can still be used at this time, but will be prohibited out in the near future.
impl VersionedNonce {
    /// Magic prefixes (first 4 bytes of `sha256(<versioned_nonce>)`) used to mark versioned nonces:
    pub const VERSIONED_MAGIC_PREFIX: [u8; 4] = hex!("5628f6c6");
}

impl TryFrom<Nonce> for VersionedNonce {
    type Error = DefuseError;

    fn try_from(nonce: Nonce) -> Result<Self> {
        let mut versioned = nonce
            .strip_prefix(&Self::VERSIONED_MAGIC_PREFIX)
            .ok_or(DefuseError::InvalidNonce)?;

        Self::deserialize_reader(&mut versioned).map_err(|_| DefuseError::InvalidNonce)
    }
}

impl From<VersionedNonce> for Nonce {
    fn from(value: VersionedNonce) -> Self {
        const SIZE: usize = size_of::<Nonce>();
        let mut result = [0u8; SIZE];

        (VersionedNonce::VERSIONED_MAGIC_PREFIX, value)
            .serialize(&mut result.as_mut_slice())
            .unwrap_or_else(|_| unreachable!());

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{Deadline, nonce::salted::Salt};
    use arbitrary::Unstructured;
    use chrono::Utc;
    use defuse_test_utils::random::random_bytes;
    use rstest::rstest;

    #[rstest]
    fn maybe_from_test(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let legacy_nonce: Nonce = u.arbitrary().unwrap();

        let expected = VersionedNonce::try_from(legacy_nonce);
        assert!(matches!(expected, Err(DefuseError::InvalidNonce)));

        let mut u = Unstructured::new(&random_bytes);
        let nonce_bytes: [u8; 15] = u.arbitrary().unwrap();
        let now = Deadline::new(Utc::now());
        let salt: Salt = u.arbitrary().unwrap();

        let salted = SaltedNonce::new(salt, ExpirableNonce::new(now, nonce_bytes));
        let nonce: Nonce = VersionedNonce::V1(salted.clone()).into();

        let exp = VersionedNonce::try_from(nonce).unwrap();
        assert_eq!(exp, VersionedNonce::V1(salted));
    }
}
