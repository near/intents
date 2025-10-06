use core::mem::size_of;
use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs};
use hex_literal::hex;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use std::io::{self, Read};

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
    Legacy(Nonce),
    V1(SaltedNonce<ExpirableNonce<[u8; 15]>>),
}

// Allowed to maintain a clean API
#[allow(clippy::fallible_impl_from)]
impl From<Nonce> for VersionedNonce {
    fn from(value: Nonce) -> Self {
        MaybeVersionedNonce::deserialize_as(&mut value.as_ref()).unwrap_or_else(|_| unreachable!())
    }
}

// Allowed to maintain a clean API
#[allow(clippy::fallible_impl_from)]
impl From<VersionedNonce> for Nonce {
    fn from(value: VersionedNonce) -> Self {
        const SIZE: usize = size_of::<Nonce>();
        let mut result = [0u8; SIZE];

        MaybeVersionedNonce::serialize_as(&value, &mut result.as_mut_slice())
            .unwrap_or_else(|_| unreachable!());

        result
    }
}

struct MaybeVersionedNonce;

impl MaybeVersionedNonce {
    /// Magic prefixes (first 4 bytes of `sha256(<versioned_nonce>)`) used to mark versioned nonces:
    pub const VERSIONED_MAGIC_PREFIX: [u8; 4] = hex!("5628f6c6");
}

impl BorshDeserializeAs<VersionedNonce> for MaybeVersionedNonce {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<VersionedNonce>
    where
        R: io::Read,
    {
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;

        let versioned = if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedNonce::deserialize_reader(reader)?
        } else {
            VersionedNonce::Legacy(Nonce::deserialize_reader(&mut prefix.chain(reader))?)
        };

        Ok(versioned)
    }
}

impl BorshSerializeAs<VersionedNonce> for MaybeVersionedNonce {
    fn serialize_as<W>(source: &VersionedNonce, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        match source {
            VersionedNonce::Legacy(nonce) => nonce.serialize(writer),
            VersionedNonce::V1(_) => (Self::VERSIONED_MAGIC_PREFIX, source).serialize(writer),
        }
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
