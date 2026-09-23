use near_sdk::near;
use serde_with::base64::Base64;

/// A deposit or withdrawal id, digested by the caller to a fixed width.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdDigest(#[serde_as(as = "Base64")] pub [u8; Self::LEN]);

impl IdDigest {
    pub const LEN: usize = 20;
}

impl From<[u8; Self::LEN]> for IdDigest {
    #[inline]
    fn from(digest: [u8; Self::LEN]) -> Self {
        Self(digest)
    }
}

impl AsRef<[u8]> for IdDigest {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A 32-byte hash of a cross-chain withdrawal payload.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PayloadHash(#[serde_as(as = "Base64")] pub [u8; 32]);

impl From<[u8; 32]> for PayloadHash {
    #[inline]
    fn from(hash: [u8; 32]) -> Self {
        Self(hash)
    }
}

impl From<PayloadHash> for [u8; 32] {
    #[inline]
    fn from(PayloadHash(hash): PayloadHash) -> Self {
        hash
    }
}

impl AsRef<[u8]> for PayloadHash {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::serde_json::{self, json};

    use super::*;

    /// A bare base64 string, not an object or a list of byte values.
    #[test]
    fn serializes_as_base64_string() {
        let json = serde_json::to_value(PayloadHash([0xAB; 32])).unwrap();
        assert_eq!(json, json!("q6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6s="));
        let back: PayloadHash = serde_json::from_value(json).unwrap();
        assert_eq!(back, PayloadHash([0xAB; 32]));
    }

    /// The 32-byte width is enforced while decoding the argument, so no call site
    /// has to re-check it.
    #[rstest::rstest]
    #[case::too_short(json!("BQUF"))]
    #[case::too_long(json!("q6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6ur"))]
    #[case::empty(json!(""))]
    #[case::not_base64(json!("!!!!"))]
    #[case::not_a_string(json!([5, 5, 5]))]
    fn rejects_non_32_byte_input(#[case] input: serde_json::Value) {
        serde_json::from_value::<PayloadHash>(input.clone())
            .expect_err(&format!("{input} should not decode"));
    }
}
