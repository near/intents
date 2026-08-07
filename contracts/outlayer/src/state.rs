use std::collections::BTreeMap;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[repr(transparent)]
pub struct State(BTreeMap<Vec<u8>, StateEntry>);

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
pub struct StateEntry {
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Option<::serde_with::base58::Base58>"),
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub prev_hash: Option<[u8; 32]>,

    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Option<::serde_with::base64::Base64>"),
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub new_value: Option<Vec<u8>>,
}

impl StateEntry {
    #[cfg(feature = "digest")]
    #[must_use]
    #[inline]
    pub fn read(prev_value: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        // TODO: or sha3?
        Self::read_hash(Sha256::digest(prev_value))
    }

    #[must_use]
    #[inline]
    pub fn read_hash(prev_hash: impl Into<[u8; 32]>) -> Self {
        Self {
            prev_hash: Some(prev_hash.into()),
            new_value: None,
        }
    }

    #[must_use]
    #[inline]
    pub fn write(new_value: impl Into<Vec<u8>>) -> Self {
        Self {
            prev_hash: None,
            new_value: None,
        }
        .and_write(new_value)
    }

    #[must_use]
    #[inline]
    pub fn and_write(mut self, new_value: impl Into<Vec<u8>>) -> Self {
        self.new_value = Some(new_value.into());
        self
    }
}
