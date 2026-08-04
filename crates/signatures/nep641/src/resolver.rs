use futures::stream::{FuturesUnordered, TryStreamExt};
use near_account_id::AccountId;
use near_kit::{BlockReference, CryptoHash, Finality, Near};
#[cfg(feature = "tracing")]
use tracing::{Span, instrument, record_all};

use crate::{AuthorizationResolution, PendingAuthorization, client::WResolveAuthArgs};

/// Offchain verifier for NEP-641 authorizations.
#[derive(Debug, Clone)]
pub struct OffchainResolver {
    client: Near,
    at_block: BlockReference,
    max_pending: usize,
    max_depth: usize,
    // state_inits: HashMap<AccountId, StateInit>,
}

impl OffchainResolver {
    /// Create new verifier with given Near client
    #[must_use]
    #[inline]
    pub const fn new(client: Near) -> Self {
        Self {
            client,
            // fetch final block by default
            at_block: BlockReference::Finality(Finality::Final),
            // allow only top-level authorizations by default
            max_pending: 0,
            max_depth: 0,
        }
    }

    /// Override block reference for [resolving](crate::AuthResolver::w_resolve_auth)
    /// **all** autorizations.
    ///
    /// **All** authorizations are resolved against the same block hash to enforce consistent
    /// state between async RPC view-calls. By default, [`.resolve_auth()`](Self::resolve_auth)
    /// fetches the `Final` block hash first and then resolves all authorizations against it.
    /// This setting overrides it and allows to resolve authorizations against the chain state
    /// from the past.
    #[must_use]
    #[inline]
    pub fn at_block(mut self, block: impl Into<BlockReference>) -> Self {
        self.at_block = block.into();
        self
    }

    // TODO: uncomment when RPC adds "pre-init" support for view-calls
    // #[inline]
    // pub fn with_state_init(mut self, state_init: impl Into<StateInit>) -> Self {
    //     let state_init = state_init.into();
    //     self.state_inits.insert(state_init.derive_account_id(), state_init);
    //     self
    // }

    /// Set an upper limit for maximum number of pending sub-authorizations to resolve.
    ///
    /// By default, this value is set to zero, so that only top-level authorizations are allowed
    /// and any sub-authorizations will fail. This is too concervative for real world use-cases,
    /// but used as a sane default to prevent from DoS attacks.
    ///
    /// Note that this doesn't change the [maximum depth](Self::with_max_depth) and it should be
    /// configured separately.
    #[must_use]
    #[inline]
    pub const fn with_max_pending(mut self, max_pending: usize) -> Self {
        self.max_pending = max_pending;
        self
    }

    /// Set an upper limit for maximum depth of sub-authorization branches.
    ///
    /// By default, this value is set to zero, so that only top-level authorizations are allowed
    /// and any sub-authorizations will fail. This is too concervative for real world use-cases,
    /// but used as a sane default to prevent from DoS attacks.
    ///
    /// Despite the implementation itself is optimized and _does not_ create a new stack frame for
    /// each sub-authorization, it's still recommended to limit the maximum depth, as each pending
    /// sub-authorization implies additional allocations and may lead to long resolution timings.
    ///
    /// This also raises the [maximum pending](Self::with_max_pending) limit to at least `max_depth`.
    #[must_use]
    #[inline]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        // this is a const version of: `self.max_pending = self.max_pending.max(self.max_depth)`
        if self.max_pending < self.max_depth {
            self.max_pending = self.max_depth;
        }
        self
    }

    /// Resolve a payload from top-level authorization according to NEP-641.
    ///
    /// This method recursively resolves given top-level authorization and all returned pending
    /// ones until no more authorizations are left, and returns a top-level authorized
    /// [payload](field@AuthorizationResolution::payload). If at least one authorization resolution
    /// fails or any [pending authorization](PendingAuthorization) resolves into a payload that
    /// doesn't match the [expected](field@PendingAuthorization::expect) one, then the whole
    /// resolution procedure is immediately aborted and an error is returned.
    ///
    /// # Block reference
    ///
    /// **All** authorizations are resolved against the same block hash to enforce consistent
    /// state between async RPC view-calls. By default, this method will fetch the `Final`
    /// block hash during top-level authorization resolution and resolve all pending ones
    /// against it.
    ///
    /// See [`.at_block()`](Self::at_block) to resolve authorizations against the chain state
    /// from the past.
    ///
    /// # Resource limits
    ///
    /// By default, only top-level authorizations are allowed and any sub-authorizations will fail.
    /// This is too concervative for real world use-cases, but used as a sane default to prevent
    /// from DoS attacks.
    ///
    /// See [`.with_max_pending()`](Self::with_max_pending) and
    /// [`.with_max_depth()`](Self::with_max_pending) to set your custom limits.
    ///
    // TODO: # Not yet initialized accounts
    /// # Legacy accounts
    ///
    // TODO
    /// If an account doesn't have a contract deployed on it or the contract doesn't implement
    /// NEP-641 standard, the implementation fallbacks to verifying offchain signature according
    /// to [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) standard.
    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(
        chain.id = self.client.chain_id().as_str(),
        account_id,
        at_block.hash,
        at_block.height,
    )))]
    pub async fn resolve_auth(
        &self,
        account_id: impl Into<AccountId>,
        authorization: String,
        
    ) -> Result<String, ResolveError> {
        let account_id = account_id.into();

        #[cfg(feature = "tracing")]
        let mut span = Span::current();
        #[cfg(feature = "tracing")]
        record_all!(span, account_id = %account_id);

        // resolve top-level authorization first
        let SingleResolved {
            mut path, // returned path already contains top-level account ID
            res:
                AuthorizationResolution {
                    payload, // top-level authorized payload
                    mut pending,
                },
            block_hash,   // resolved block hash
            block_height, // resolved block height
        } = self
            .resolve_single(
                account_id,
                vec![], // path is empty for top-level authorization
                authorization,
                self.at_block.clone(),
            )
            .await?;

        #[cfg(feature = "tracing")]
        // update `at_block.hash` with resolved block hash
        record_all!(span, at_block.hash = %block_hash, at_block.height = block_height);

        // keep track of number of pending sub-authorizations we're resolving
        let mut pending_left = self.max_pending;
        // a pool of futures to resolve all pending sub-authorizations concurrently
        let mut in_flight = FuturesUnordered::new();

        loop {
            // check if new path exceeds max depth limit for pending sub-authorizations, if any
            if !pending.is_empty() && path.len() > self.max_depth {
                return Err(ResolveError::MaxDepthExceeded(self.max_depth));
            }

            // check if adding new pending sub-authorizations wouldn't exceed max pending limit
            pending_left = pending_left
                .checked_sub(pending.len())
                .ok_or(ResolveError::TooManyPending(self.max_pending))?;

            // add pending sub-authorizations to the in-flight pool
            in_flight.extend(pending.into_iter().map(|pending| {
                self.resolve_pending(
                    path.clone(), // path already contains parent resolver ID
                    pending,
                    block_hash, // resolve pending authorizations at the same block hash
                    #[cfg(feature = "tracing")]
                    span.clone(),
                )
            }));

            // wait until a pending sub-authorization resolves, if any
            let Some(resolved) = in_flight.try_next().await? else {
                // no more authorizations left, return the top-level output
                return Ok(payload);
            };

            // overwrite `path` and `pending` from the resolved sub-authorization
            PendingResolved {
                path, // path already contains parent resolver ID
                pending,
                #[cfg(feature = "tracing")]
                span,
            } = resolved;
        }
    }

    /// Resolve pending sub-authorization and check that returned payload matches the expected one.
    #[cfg_attr(feature = "tracing", instrument(
        level = "DEBUG",
        parent = parent_span,
        skip_all,
        fields(
            chain.id = self.client.chain_id().as_str(),
            account_id = %pending.account_id,
            at_block.hash = %block_hash,
        ),
    ))]
    async fn resolve_pending(
        &self,
        path: Vec<AccountId>,
        pending: PendingAuthorization,
        block_hash: CryptoHash,
        #[cfg(feature = "tracing")] parent_span: Span,
    ) -> Result<PendingResolved, ResolveError> {
        let SingleResolved { path, res, .. } = self
            .resolve_single(pending.account_id, path, pending.authorization, block_hash)
            .await?;

        // check that returned payload matches the expected one
        if res.payload != pending.expect {
            return Err(ResolveError::InvalidPayload(path));
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(payload = res.payload, "resolved");

        Ok(PendingResolved {
            path,
            pending: res.pending,
            #[cfg(feature = "tracing")]
            span: Span::current(),
        })
    }

    /// Resolve a single authorization
    async fn resolve_single(
        &self,
        account_id: AccountId,
        mut path: Vec<AccountId>,
        authorization: String,
        block: impl Into<BlockReference>,
    ) -> Result<SingleResolved, ResolveError> {
        let res = self
            .client
            .rpc()
            .view_function(
                &account_id,
                "w_resolve_auth",
                &serde_json::to_vec(&WResolveAuthArgs {
                    path: &path,
                    authorization: &authorization,
                })
                .expect("JSON: serialization failed"),
                block.into(),
                // TODO: "pre-init" if we have StateInit for this AccountId
                // self.state_inits.get(&msg.resolver_id),
            )
            // TODO: handle contract errors
            .await?;

        // // if was set, make sure RPC returned same block hash
        // if let Some(at_block_hash) = at_block_hash
        //     && at_block_hash != res.block_hash
        // {
        //     // TODO: RPCs can be behind a load-balancer, so that they can return UnknownBlock error
        //     // TODO: maybe we need to retry with minimum-known block_height?
        //     return Err(
        //         near_kit::RpcError::InvalidResponse("block hash mismatch".to_string()).into(),
        //     );
        // }

        // TODO: if the contract doesn't implement NEP-641, then fallback to
        // HARDCODED signature verification algorithms for:
        // * each FullAccessKey currently added to EXISTING account
        // * `public_keys` from UniversalStateInit for NON-EXISTING account
        // * implicit public key for NON-EXISTING Near/eth implicit AccountIds
        //
        // TODO: what if `w_resolve_auth()` failed, but the account also has
        // FullAccessKeys on it - do we need to fallback to them, too?
        //
        // TODO: fallback to NEP-413 (for all cases above?)
        // TODO: fallback to intents.near(far?) as resolver_id?

        // append the account ID to path for pending sub-authorizations, if any
        path.push(account_id);

        Ok(SingleResolved {
            path,
            res: res.json()?,
            block_hash: res.block_hash,
            block_height: res.block_height,
        })
    }
}

/// Resolved pending sub-authorization
struct PendingResolved {
    /// Path for pending sub-authorizations, if any.
    path: Vec<AccountId>,

    /// Optional list of pending sub-authorizations
    pending: Vec<PendingAuthorization>,

    /// Span where this authorization was resolved
    #[cfg(feature = "tracing")]
    span: Span,
}

/// A single resolved authorization
struct SingleResolved {
    /// Path for pending sub-authorizations, if any.
    path: Vec<AccountId>,

    /// Authorization resolution
    res: AuthorizationResolution,

    /// Resolved block hash
    block_hash: CryptoHash,

    /// Resolved block height
    block_height: u64,
}

/// An error returned by [`OffchainResolver`]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveError {
    #[error(
        "invalid payload resolved at path: {}",
        .0.iter().map(ToString::to_string).collect::<Vec<_>>().join(" -> "),
    )]
    InvalidPayload(Vec<AccountId>),

    #[error("max depth exceeded, maximum is set to: {0}")]
    MaxDepthExceeded(usize),

    #[error(transparent)]
    Near(#[from] near_kit::Error),

    #[error("too many pending authorizations, maximum is set to: {0}")]
    TooManyPending(usize),
}

impl From<near_kit::RpcError> for ResolveError {
    #[inline]
    fn from(err: near_kit::RpcError) -> Self {
        Self::Near(err.into())
    }
}

impl From<serde_json::Error> for ResolveError {
    #[inline]
    fn from(err: serde_json::Error) -> Self {
        Self::Near(err.into())
    }
}
