use std::borrow::Cow;

use near_account_id::AccountIdRef;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,

    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],

    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub approved_hash: [u8; 32],
}

impl<'a> State<'a> {
    pub const STATE_KEY: &'static [u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0; 32];

    #[inline]
    pub fn owner(owner_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            owner_id: owner_id.into(),
            code_hash: Self::DEFAULT_HASH,
            approved_hash: Self::DEFAULT_HASH,
        }
    }

    #[must_use]
    #[inline]
    pub fn with_index(mut self, index: u32) -> Self {
        self.code_hash = [0u8; 32];
        self.code_hash[32 - 4..].copy_from_slice(&index.to_be_bytes());
        self
    }

    #[must_use]
    #[inline]
    pub fn pre_approve(mut self, hash: impl Into<[u8; 32]>) -> Self {
        self.approved_hash = hash.into();
        self
    }

    #[cfg(feature = "digest")]
    #[must_use]
    #[inline]
    pub fn pre_approve_code(self, code: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        self.pre_approve(Sha256::digest(code))
    }

    #[cfg(feature = "borsh")]
    pub fn as_storage(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
