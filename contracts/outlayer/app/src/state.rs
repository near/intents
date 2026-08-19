use std::borrow::Cow;

use near_account_id::AccountIdRef;

/// State of an [`OutlayerApp`](crate::contract::OutlayerApp) contract
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
    /// Admin account ID.
    pub admin_id: Cow<'a, AccountIdRef>,

    /// Currently approved SHA-256 hash of the code.
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],

    /// URL where the binary can be fetched from.
    pub code_url: Cow<'a, str>,
}

impl<'a> State<'a> {
    pub const STATE_KEY: &'static [u8] = b"";

    /// Create new state with given admin ID.
    ///
    /// Code [hash](field@Self::code_hash) starts as all-zero bytes and [URL](field@Self::code_url)
    /// as empty string. Both are expected to be set via [`.with_code()`](Self::with_code) /
    /// [`.with_code_hash()`](Self::with_code_hash) and
    /// [`.with_code_url()`](Self::with_code_url) before deriving an address.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_outlayer_app::State;
    /// # use hex_literal::hex;
    /// # use near_account_id::AccountIdRef;
    /// # const ADMIN_ID: &AccountIdRef = AccountIdRef::new_or_panic("admin.near");
    /// let state = State::new(ADMIN_ID);
    ///
    /// assert_eq!(*state.admin_id, *ADMIN_ID);
    /// assert_eq!(state.code_hash, [0u8; 32]);
    /// assert_eq!(state.code_url, "");
    /// ```
    #[must_use]
    #[inline]
    pub fn new(admin_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash: [0u8; 32],
            code_url: Cow::Owned(String::new()),
        }
    }

    /// Set [`field@Self::code_hash`] to the SHA-256 digest of given code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_outlayer_app::State;
    /// # use hex_literal::hex;
    /// # use near_account_id::AccountIdRef;
    /// # const ADMIN_ID: &AccountIdRef = AccountIdRef::new_or_panic("admin.near");
    /// let wasm = b"TODO"; // read from file
    /// let state = State::new(ADMIN_ID).with_code(wasm);
    ///
    /// assert_eq!(
    ///     state.code_hash,
    ///     hex!("337e547a950fc8a98592f10d964c1e79a304961790a8da0ce449a1f000cefabb"),
    /// )
    /// ```
    #[cfg(feature = "digest")]
    #[must_use]
    #[inline]
    pub fn with_code(self, code: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        self.with_code_hash(Sha256::digest(code))
    }

    /// Set [`field@Self::code_hash`] to the given value.
    #[must_use]
    #[inline]
    pub fn with_code_hash(mut self, code_hash: impl Into<[u8; 32]>) -> Self {
        self.code_hash = code_hash.into();
        self
    }

    /// Overwrite the [URL](field@Self::code_url) where the code binary can be fetched from.
    #[must_use]
    #[inline]
    pub fn with_code_url(mut self, code_url: impl Into<Cow<'a, str>>) -> Self {
        self.code_url = code_url.into();
        self
    }

    /// Construct storage key-value pairs for `StateInit` of an
    /// [`OutlayerApp`](crate::contract::OutlayerApp) contract.
    #[cfg(feature = "borsh")]
    #[inline]
    pub fn as_storage(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).expect("borsh"),
        )]
        .into()
    }
}
