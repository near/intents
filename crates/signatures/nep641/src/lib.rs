#[cfg(feature = "access-keys")]
pub mod access_keys;
#[cfg(feature = "near-kit")]
pub mod client;
mod message;
#[cfg(feature = "resolver")]
pub mod resolver;

pub use self::message::*;

pub use defuse_time::Timestamp;
use near_account_id::AccountId;

/// A smart-contract implementing NEP-641 interface.
pub trait AuthResolver {
    /// A view-method to resolve [offchain](#offchain-only) authorization according to NEP-641.
    ///
    /// The implementation SHOULD resolve given `authorization` blob along with [`path`](#path)
    /// and return an authorized [`payload`](field@AuthorizationResolution::payload) along with
    /// an _optional_ list of [pending sub-authorizations](PendingAuthorization). The authorized
    /// payload SHOULD be accepted by the offchain resolver if and only if **all** pending
    /// sub-authorizations resolve successfully into corresponding
    /// [expected](PendingAuthorization::expect) payloads.
    ///
    /// # Panics
    ///
    /// The implementation MUST panic if the authorization is invalid. The panic SHOULD include
    /// informative message explaining the failure reason.
    ///
    /// # Path
    ///
    /// Offchain verifier starts the verification procedure from the top-level authorization with
    /// empty `path`. If the authorization resolves successfully and the pending sub-authorizations
    /// list is not empty, then the current resolver contract ID is _appended to the end_ of `path`
    /// and the latter gets propagated to all sub-resolvers.
    ///
    /// Thus, for any non-top-level resolver the `path` argument includes the whole traversal path
    /// _starting_ from the top-level resolver and _ending_ with the closest ancestor (i.e. parent),
    /// which has triggered the current `w_resolve_auth()`:
    ///
    /// <table>
    ///   <thead>
    ///     <tr>
    ///       <th>Depth</th>
    ///       <th>Resolver ID</th>
    ///       <th>Args</th>
    ///       <th>Return</th>
    ///     </tr>
    ///   </thead>
    /// <tbody>
    /// <tr>
    /// <td align="right">0</td>
    /// <td><code>resolver.near</code></td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "path": [],
    ///   "authorization": "auth0"
    /// }
    /// ```
    ///
    /// </td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "payload": "payload0",
    ///   "pending": [{
    ///     "account_id": "sub.resolver.near",
    ///     "authorization": "auth1",
    ///     "expect": "payload1"
    ///   }]
    /// }
    /// ```
    ///
    /// </td>
    /// </tr>
    /// <tr>
    /// <td align="right">1</td>
    /// <td><code>sub.resolver.near</code></td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "path": ["resolver.near"],
    ///   "authorization": "auth1"
    /// }
    /// ```
    ///
    /// </td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "payload": "payload1",
    ///   "pending": [{
    ///     "account_id": "sub.sub.resolver.near",
    ///     "authorization": "auth2",
    ///     "expect": "payload2"
    ///   }]
    /// }
    /// ```
    ///
    /// </td>
    /// </tr>
    /// <tr>
    /// <td align="right">2</td>
    /// <td><code>sub.sub.resolver.near</code></td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "path": ["resolver.near", "sub.resolver.near"],
    ///   "authorization": "auth2"
    /// }
    /// ```
    ///
    /// </td>
    /// <td>
    ///
    /// ```json
    /// {
    ///   "payload": "payload2"
    /// }
    /// ```
    ///
    /// </td>
    /// </tr>
    /// </tbody>
    /// </table>
    ///
    /// # Cycles
    ///
    /// Cycles between sub-resolvers are _allowed_ only as long as they are _finite_. However,
    /// keep in mind that offchain verifiers MAY impose limits on the number and/or recursion
    /// depth for pending sub-authorizations to prevent from DoS attacks and reject long
    /// sub-authorizations chains.
    ///
    /// # Offchain Only
    ///
    /// <div class="warning">
    ///
    /// **DO NOT** call this view-method in on-chain transactions!
    ///
    /// </div>
    ///
    /// NEP-641 standard is **NOT** designed to be used for on-chain transfer "approvals"
    /// or any other actions that modify state of the blockchain. Offchain messages are
    /// intended to be verified _only_ offchain as they don't mutate any state and, hence,
    /// cannot prevent replay attacks.
    ///
    /// Instead, use on-chain messages (e.g. request messages, transactions, delegate actions),
    /// which are specifically designed with replay-protection mechanism in mind.
    fn w_resolve_auth(
        &self,
        path: Vec<AccountId>,
        authorization: String,
    ) -> AuthorizationResolution;
}

/// Authorization resolution returned from [`w_resolve_auth()`](AuthResolver::w_resolve_auth)
/// method.
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthorizationResolution {
    /// A payload that was successfully authorized from given authorization blob
    /// at the current contract state.
    // TODO: docs: in case of wallets/DAOs this should be Message
    // TODO: better naming?
    pub payload: String,

    /// Optional list of pending sub-authorizations that MUST be successfully
    /// [resolved](AuthResolver::w_resolve_auth) before accepting the authorized
    /// [payload](field@Self::payload).
    ///
    /// If emtpy, then this authorization is a leaf that terminates the current branch.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub pending: Vec<PendingAuthorization>,
}

impl AuthorizationResolution {
    /// Create a leaf authorization resolution with given payload.
    ///
    /// See [`.add_pending()`](Self::add_pending) to add pending sub-authorizations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::AuthorizationResolution;
    /// let auth = AuthorizationResolution::new("payload");
    /// assert!(auth.is_leaf());
    /// ```
    #[inline]
    pub fn new(payload: impl Into<String>) -> Self {
        Self {
            payload: payload.into(),
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
    /// let auth = AuthorizationResolution::new("payload")
    ///     .add_pending(
    ///         AccountIdRef::new_or_panic("sub.resolver.near"),
    ///         "auth1",
    ///         "payload1",
    ///     );
    /// assert!(!auth.is_leaf());
    /// ```
    #[inline]
    pub fn add_pending(
        mut self,
        account_id: impl Into<AccountId>,
        authorization: impl Into<String>,
        expect: impl Into<String>,
    ) -> Self {
        self.pending.push(PendingAuthorization {
            account_id: account_id.into(),
            authorization: authorization.into(),
            expect: expect.into(),
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
    ///     AccountIdRef::new_or_panic("sub.resolver.near"),
    ///     "auth",
    ///     "payload",
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

/// A [pending](field@AuthorizationResolution::pending) sub-authorization.
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
    /// Account ID to [resolve](AuthResolver::w_resolve_auth) the
    /// [sub-authorization](field@Self::authorization) on.
    pub account_id: AccountId,

    /// Authorization blob to pass to [`w_resolve_auth()`](AuthResolver::w_resolve_auth)
    /// method on the [sub-resolver](field@Self::account_id).
    pub authorization: String,

    /// Expected authorized [`payload`](field@AuthorizationResolution::payload) to be
    /// successfully [resolved](AuthResolver::w_resolve_auth) by the
    /// [sub-resolver](field@Self::account_id) from this
    /// [sub-authorization](field@Self::authorization).
    ///
    /// If the pending authorization resolution fails or returns a different payload, then
    /// the whole verification procedure MUST fail immediately and top-level authorization
    /// MUST be considered invalid.
    pub expect: String,
}
