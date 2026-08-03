use futures::stream::{FuturesUnordered, TryStreamExt};
use near_account_id::AccountId;
use near_kit::{BlockReference, CryptoHash, Finality, Near};
#[cfg(feature = "tracing")]
use tracing::{Span, field, instrument, record_all};

use crate::{AuthorizationResolution, PendingAuthorization, client::WResolveAuthArgs};

/// Offchain verifier for NEP-641 authorizations.
#[derive(Debug, Clone)]
pub struct OffchainVerifier {
    client: Near,
    at_block_hash: Option<CryptoHash>,
    max_pending: usize,
    max_depth: usize,
    // TODO
    // max_depth: usize,
    // // TODO
    // bound_in_flight: usize,
    // state_inits: HashMap<AccountId, StateInit>,
}

impl OffchainVerifier {
    /// Create new verifier with given Near client
    #[must_use]
    #[inline]
    pub const fn new(client: Near) -> Self {
        // TODO: check client.rpc().status().await?.chain_id
        Self {
            client,
            // fetch final block hash by default
            at_block_hash: None,
            // unbounded by default
            // TODO: set reasonable default
            max_pending: 0,
            max_depth: 0,
        }
    }

    // TODO: uncomment when RPC adds "pre-init" support for view-calls
    // #[inline]
    // pub fn with_state_init(mut self, state_init: impl Into<StateInit>) -> Self {
    //     let state_init = state_init.into();
    //     self.state_inits.insert(state_init.derive_account_id(), state_init);
    //     self
    // }

    /// Set an upper limit for maxumim number of pending sub-authorizations to resolve.
    ///
    /// By default, only top-level authorizations are allowed and this
    /// value is set to zero to prevent from DoS attacks.
    #[must_use]
    #[inline]
    pub const fn with_max_pending(mut self, max_pending: usize) -> Self {
        self.max_pending = max_pending;
        self
    }

    /// Set an upper limit for maximum depth of sub-authorization branches.
    ///
    /// This also raises the [maximum pending](Self::with_max_pending) limit
    /// to at least `max_depth`.
    ///
    /// By default, only top-level authorizations are allowed and this
    /// value is set to zero to prevent from DoS attacks.
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

    /// Override block hash for [resolving](crate::AuthResolver::w_resolve_auth)
    /// **all** autorizations.
    ///
    /// **All** authorizations are resolved against the same block hash to enforce
    /// consistent state between async RPC view-calls. By default,
    /// [`.resolve_auth()`](Self::resolve_auth) fetches the `Final` block
    /// hash first and then resolves all authorizations against it. This setting overrides
    /// it and allows to resolve authorizations against the chain state from the past.
    #[must_use]
    #[inline]
    pub fn at_block_hash(mut self, fixed_block_hash: impl Into<CryptoHash>) -> Self {
        self.at_block_hash = Some(fixed_block_hash.into());
        self
    }

    /// Verify top-level authorization according to NEP-641.
    ///
    /// This method recursively calls
    /// [`w_resolve_auth(msg, proof)`](crate::contract::OffchainAuthorizer::w_resolve_auth)
    /// view-method on [`msg.resolver_id`](field@OffchainMessage::resolver_id) and all returned
    /// sub-accounts until no more pending autorizations are left.
    ///
    /// # Result
    ///
    /// If all view-calls return successfully, then `Ok(())` is returned and the top-level
    /// authorization is considered valid.
    ///
    /// If at least one view-call doesn't return successfully, then the top-level autorization
    /// is considered invalid: all pending view-calls are immediatelly aborted and an error is
    /// returned.
    ///
    /// # Block reference
    ///
    /// **All** authorizations are resolved against the same block hash to enforce consistent
    /// state between async RPC view-calls. By default, this method will fetch the `Final`
    /// block hash along with top-level [`w_resolve_auth()`](crate::contract::OffchainAuthorizer::w_resolve_auth)
    /// and resolve all pending authorizations against it.
    ///
    /// See [`.at_block_hash()`](Self::at_block_hash) to resolve authorizations against
    /// the chain state from the past.
    ///
    // TODO: # Not yet initialized accounts
    /// # Legacy accounts
    ///
    /// If a contract doesn't implement NEP-641 standard, the implementation fallbacks to verifying
    /// offchain signature according to [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
    /// standard.
    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(
        // TODO
        // %account_id,
        // %msg.resolver_id,
        // %msg.signer_id,
        // %msg.chain_id,
        // msg.hash = %bs58::encode(msg.hash()).into_string(),
        at_block.hash, // will be recorded after top-level resolve
    )))]
    // TODO: return signer_id? but this doesn't force the caller
    // to check the actual message being signed...
    pub async fn resolve_auth(
        &self,
        account_id: impl Into<AccountId>, // TODO: is it not Send?
        authorization: String,
    ) -> Result<String, ResolveError> {
        let SingleResolved {
            mut path,
            resolution:
                AuthorizationResolution {
                    authorized,
                    mut pending,
                },
            block_hash,
        } = self
            .resolve_single(
                account_id.into(),
                // path is empty for top-level authorization
                vec![],
                authorization,
                // if set, resolve top-level authorization at fixed block hash,
                // or final otherwise
                self.at_block_hash
                    .map_or(BlockReference::Finality(Finality::Final), Into::into),
            )
            .await?;

        #[cfg(feature = "tracing")]
        record_all!(Span::current(), at_block.hash = %block_hash);

        let mut pending_left = self.max_pending;
        // TODO: .buffer_unordered()
        let mut in_flight = FuturesUnordered::new();

        loop {
            if !pending.is_empty() && path.len() > self.max_depth {
                return Err(ResolveError::MaxDepthExceeded(self.max_depth));
            }

            pending_left = pending_left
                .checked_sub(pending.len())
                .ok_or(ResolveError::TooManyAuthorizations(self.max_pending))?;

            in_flight.extend(
                pending
                    .into_iter()
                    // TODO: inspect tracing
                    .map(|pending| {
                        self.resolve_pending(
                            // propagate receiver to sub-authorization
                            path.clone(),
                            pending,
                            // resolve pending authorizations at the same block hash
                            block_hash,
                        )
                    }),
            );

            let Some(resolved) = in_flight.try_next().await? else {
                // no more authorizations left, return the top-level output
                return Ok(authorized);
            };

            PendingResolved { path, pending } = resolved;
        }
    }

    // TODO: tracing?
    async fn resolve_pending(
        &self,
        path: Vec<AccountId>,
        pending: PendingAuthorization,
        block: impl Into<BlockReference>,
    ) -> Result<PendingResolved, ResolveError> {
        let SingleResolved {
            path, resolution, ..
        } = self
            .resolve_single(pending.account_id, path, pending.authorization, block)
            .await?;

        if resolution.authorized != pending.expect {
            return Err(ResolveError::InvalidOutput);
        }

        Ok(PendingResolved {
            path,
            pending: resolution.pending,
        })
    }

    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(
        // TODO
        // %receiver_id,
        // %msg.resolver_id,
        // %msg.signer_id,
        // %msg.chain_id,
        // msg.hash = %bs58::encode(msg.hash()).into_string(),
        // at_block.hash = at_block_hash.map(field::display),
    )))]
    async fn resolve_single(
        &self,
        account_id: AccountId,
        mut path: Vec<AccountId>,
        authorization: String,
        block: impl Into<BlockReference>,
    ) -> Result<SingleResolved, ResolveError> {
        // if msg.chain_id != self.client.chain_id().as_str() {
        //     return Err(ResolveError::InvalidChainId);
        // }

        let res = self
            .client
            .rpc()
            .view_function(
                &account_id,
                "w_resolve_auth",
                &serde_json::to_vec(&WResolveAuthArgs {
                    path: &path,
                    input: &authorization,
                })
                .expect("JSON: serialization failed"),
                block.into(),
                // TODO: "pre-init" if we StateInit for this AccountId
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

        // append the account ID to path for pending sub-authorizations
        path.push(account_id);

        Ok(SingleResolved {
            path,
            resolution: res.json()?,
            block_hash: res.block_hash,
        })
    }
}

struct PendingResolved {
    path: Vec<AccountId>,
    pending: Vec<PendingAuthorization>,
}

// TODO: rename?
struct SingleResolved {
    path: Vec<AccountId>,
    resolution: AuthorizationResolution,
    block_hash: CryptoHash,
}

/// An error returned by [`OffchainResolver`]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveError {
    // TODO: better naming
    #[error("invalid")]
    InvalidOutput,

    #[error("max depth exceeded, maximum is set to: {0}")]
    MaxDepthExceeded(usize),

    #[error(transparent)]
    Near(#[from] near_kit::Error),

    #[error("too many pending authorizations, maximum is set to: {0}")]
    TooManyAuthorizations(usize),
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
