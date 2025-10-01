use hex_literal::hex;
use near_sdk::borsh::BorshDeserialize;
use near_sdk::borsh::BorshSerialize;
use std::io::{self, Read};

use crate::{
    Nonce,
    nonce::{expirable::ExpirableNonce, salted::SaltedNonce},
};

/// To distinguish between legacy nonces and versioned nonces
/// we use a specific prefix individual for each version.
/// Versioned nonce formats:
/// - Legacy: plain `[u8; 32]`
/// - V1: `V1_MAGIC_PREFIX (4 bytes) || SALT (4 bytes) || DEADLINE (8 bytes) || NONCE (16 random bytes)`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionedNonce {
    Legacy(Nonce),
    V1(SaltedNonce<ExpirableNonce<[u8; 16]>>),
}

impl VersionedNonce {
    /// Magic prefixes (first 4 bytes of `sha256(<nonce_vN>)`) used to mark versioned nonces:
    pub const V1_MAGIC_PREFIX: [u8; 4] = hex!("a727892c");

    /// Returns the version prefix, if any
    pub const fn prefix(&self) -> Option<[u8; 4]> {
        match self {
            Self::Legacy(_) => None,
            Self::V1(_) => Some(Self::V1_MAGIC_PREFIX),
        }
    }
}

impl TryFrom<Nonce> for VersionedNonce {
    type Error = io::Error;

    fn try_from(value: Nonce) -> Result<Self, Self::Error> {
        Self::deserialize(&mut value.as_ref())
    }
}

impl TryFrom<VersionedNonce> for Nonce {
    type Error = io::Error;

    fn try_from(value: VersionedNonce) -> io::Result<Self> {
        // Serialize into a Vec first and validate the exact layout.
        let mut buf = Vec::with_capacity(32);
        value.serialize(&mut buf)?;

        if buf.len() != 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "encoded VersionedNonce has unexpected length: {}",
                    buf.len()
                ),
            ));
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(&buf);
        Ok(result)
    }
}
impl BorshDeserialize for VersionedNonce {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;

        let versioned = match prefix {
            Self::V1_MAGIC_PREFIX => Self::V1(
                SaltedNonce::<ExpirableNonce<[u8; 16]>>::deserialize_reader(reader)?,
            ),
            _ => Self::Legacy(Nonce::deserialize_reader(&mut prefix.chain(reader))?),
        };

        Ok(versioned)
    }
}

impl BorshSerialize for VersionedNonce {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        // TODO: make it more concise
        if let Some(prefix) = self.prefix() {
            writer.write_all(&prefix)?;
        }

        match self {
            Self::Legacy(nonce) => nonce.serialize(writer),
            Self::V1(salted) => salted.serialize(writer),
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

        let expected = VersionedNonce::try_from(nonce).expect("unable to convert nonce");
        assert_eq!(expected, VersionedNonce::Legacy(nonce));
    }

    #[rstest]
    fn v1_roundtrip_layout(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let nonce_bytes: [u8; 16] = u.arbitrary().unwrap();
        let now = Deadline::new(Utc::now());
        let salt: Salt = u.arbitrary().unwrap();

        let salted = SaltedNonce::new(salt, ExpirableNonce::new(now, nonce_bytes));
        let nonce: Nonce = VersionedNonce::V1(salted.clone()).try_into().unwrap();
        let exp = VersionedNonce::try_from(nonce).expect("unable to convert nonce");

        assert_eq!(exp, VersionedNonce::V1(salted));
    }
}
