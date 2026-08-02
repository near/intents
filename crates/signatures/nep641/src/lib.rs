#[cfg(feature = "near-kit")]
pub mod client;
pub mod contract;
mod message;
#[cfg(feature = "resolver")]
pub mod resolver;

pub use self::message::*;

use near_account_id::AccountId;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthorizationResolution {
    // TODO: rename to "authorized"?
    pub output: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub pending: Vec<PendingAuthorization>,
}

impl AuthorizationResolution {
    /// Create a leaf authorization resolution.
    ///
    /// See [`.add_pending()`](Self::add_pending) to add pending ones.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::AuthorizationResolution;
    /// let auth = AuthorizationResolution::new("output");
    /// assert!(auth.is_leaf());
    /// ```
    #[inline]
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            pending: Vec::new(),
        }
    }

    /// Add a pending downstream authorization resolution on given account ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use near_account_id::AccountIdRef;
    /// # use defuse_nep641::AuthorizationResolution;
    /// let auth = AuthorizationResolution::new("output")
    ///     .add_pending(
    ///         AccountIdRef::new_or_panic("pending.near"),
    ///         "input",
    ///         "output",
    ///     );
    /// assert!(!auth.is_leaf());
    /// ```
    #[inline]
    pub fn add_pending(
        mut self,
        account_id: impl Into<AccountId>,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        self.pending.push(PendingAuthorization {
            account_id: account_id.into(),
            input: input.into(),
            output: output.into(),
        });
        self
    }

    /// Returns whether this authorization resolution is a leaf, i.e. doesn't
    /// have any pending ones.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use near_account_id::AccountIdRef;
    /// # use defuse_nep641::AuthorizationResolution;
    /// let leaf = AuthorizationResolution::new("output");
    /// assert!(leaf.is_leaf());
    ///
    /// let intermediate = leaf.add_pending(
    ///     AccountIdRef::new_or_panic("pending.near"),
    ///     "input",
    ///     "output",
    /// );
    /// assert!(!intermediate.is_leaf());
    /// ```
    #[inline]
    pub const fn is_leaf(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Extend<PendingAuthorization> for AuthorizationResolution {
    #[inline]
    fn extend<T: IntoIterator<Item = PendingAuthorization>>(&mut self, iter: T) {
        self.pending.extend(iter);
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingAuthorization {
    // TODO: returning self should be not allowed
    pub account_id: AccountId,
    // TODO: method like in JSON-RPC?
    pub input: String,
    // TODO: can we avoid fixing the output?
    // callback pattern: ft::w_resolve_auth("tell me how many tokens a user has and call me back with the number + this string")
    pub output: String,
}
