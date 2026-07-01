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
/// State of a Global Deployer contract
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

    /// Create new state with given `owner_id`.
    #[inline]
    pub fn owner(owner_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            owner_id: owner_id.into(),
            code_hash: Self::DEFAULT_HASH,
            approved_hash: Self::DEFAULT_HASH,
        }
    }

    /// Overwrite [`field@Self::code_hash`] with given index.
    ///
    /// This can be used to derive multiple deployers for a single owner.
    #[must_use]
    #[inline]
    pub fn with_index(mut self, index: u32) -> Self {
        self.code_hash = [0u8; 32];
        self.code_hash[32 - 4..].copy_from_slice(&index.to_be_bytes());
        self
    }

    /// Pre-approve given SHA-256 code hash, so that first `gd_deploy()`
    /// won't require `gd_approve()` before it.
    #[must_use]
    #[inline]
    pub fn pre_approve(mut self, hash: impl Into<[u8; 32]>) -> Self {
        self.approved_hash = hash.into();
        self
    }

    #[cfg(feature = "digest")]
    /// Pre-approve given code
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_global_deployer_core::State;
    /// # use hex_literal::hex;
    /// # use near_account_id::AccountIdRef;
    /// # const OWNER_ID: &AccountIdRef = AccountIdRef::new_or_panic("owner.near");
    /// let wasm = b"TODO"; // read from file
    /// let state = State::owner(OWNER_ID).pre_approve_code(wasm);
    ///
    /// assert_eq!(
    ///     state.approved_hash,
    ///     hex!("337e547a950fc8a98592f10d964c1e79a304961790a8da0ce449a1f000cefabb"),
    /// )
    /// ```
    #[must_use]
    #[inline]
    pub fn pre_approve_code(self, code: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        self.pre_approve(Sha256::digest(code))
    }

    #[cfg(feature = "borsh")]
    /// Construct storage key-value pairs for `StateInit`
    /// of Global Deployer contract.
    pub fn as_storage(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
