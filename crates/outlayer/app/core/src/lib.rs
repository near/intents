use std::borrow::Cow;

use near_account_id::AccountIdRef;

#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
/// State of an Outlayer App contract
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<'a> {
    pub admin_id: Cow<'a, AccountIdRef>,

    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],

    pub code_url: Cow<'a, str>,
}

impl<'a> State<'a> {
    pub const STATE_KEY: &'static [u8] = b"";

    /// Create new state with given `admin_id`.
    ///
    /// [`field@Self::code_hash`] starts as all-zero bytes and
    /// [`field@Self::code_url`] as an empty string; both are expected to be
    /// set via [`Self::with_code_hash`]/[`Self::with_code_url`] before
    /// deriving an address.
    #[inline]
    pub fn new(admin_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash: [0u8; 32],
            code_url: Cow::Borrowed(""),
        }
    }

    /// Overwrite the approved SHA-256 code hash.
    #[must_use]
    #[inline]
    pub fn with_code_hash(mut self, code_hash: impl Into<[u8; 32]>) -> Self {
        self.code_hash = code_hash.into();
        self
    }

    /// Overwrite the URL where the code binary can be fetched from.
    #[must_use]
    #[inline]
    pub fn with_code_url(mut self, code_url: impl Into<Cow<'a, str>>) -> Self {
        self.code_url = code_url.into();
        self
    }

    #[cfg(feature = "digest")]
    /// Set [`field@Self::code_hash`] to the SHA-256 digest of given code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_outlayer_app_core::State;
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
    #[must_use]
    #[inline]
    pub fn with_code(self, code: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        self.with_code_hash(Sha256::digest(code))
    }

    #[cfg(feature = "borsh")]
    /// Construct storage key-value pairs for `StateInit`
    /// of an Outlayer App contract.
    pub fn as_storage(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
