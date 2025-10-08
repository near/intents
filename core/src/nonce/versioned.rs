use hex_literal::hex;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    Nonce,
    nonce::{expirable::ExpirableNonce, salted::SaltedNonce},
};

/// To distinguish between legacy nonces and versioned nonces
/// we use a specific prefix individual for each version.
/// Versioned nonce formats:
/// - Legacy: plain `[u8; 32]`
/// - VERSIONED: `VERSIONED_MAGIC_PREFIX (4 bytes) || VERSION (1 byte) || NONCE_BYTES (27 bytes)`:
///     - V1: `SALT (4 bytes) || DEADLINE (8 bytes) || NONCE (15 random bytes)`
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub enum VersionedNonce {
    V1(SaltedNonce<ExpirableNonce<[u8; 15]>>),
}

impl VersionedNonce {
    /// Magic prefixes (first 4 bytes of `sha256(<versioned_nonce>)`) used to mark versioned nonces:
    pub const VERSIONED_MAGIC_PREFIX: [u8; 4] = hex!("5628f6c6");

    pub fn maybe_from(n: Nonce) -> Option<Self> {
        let mut versioned = n.strip_prefix(&Self::VERSIONED_MAGIC_PREFIX)?;
        Self::deserialize_reader(&mut versioned).ok()
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
    fn legacy_roundtrip_layout(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonce: Nonce = u.arbitrary().unwrap();

        let expected = VersionedNonce::from(nonce);
        assert_eq!(expected, VersionedNonce::Legacy(nonce));

        let back = Nonce::from(expected);
        assert_eq!(back, nonce);
    }

    #[rstest]
    fn v1_roundtrip_layout(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonce_bytes: [u8; 15] = u.arbitrary().unwrap();
        let now = Deadline::new(Utc::now());
        let salt: Salt = u.arbitrary().unwrap();

        let salted = SaltedNonce::new(salt, ExpirableNonce::new(now, nonce_bytes));
        let nonce: Nonce = VersionedNonce::V1(salted.clone()).into();
        let exp = VersionedNonce::from(nonce);

        assert_eq!(exp, VersionedNonce::V1(salted));

        let back = Nonce::from(exp);
        assert_eq!(back, nonce);
    }
}
