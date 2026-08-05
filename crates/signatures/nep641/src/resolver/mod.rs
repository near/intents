mod access_key;
mod contract;
mod error;

pub use self::{access_key::*, error::*};

use futures::{
    join,
    stream::{FuturesUnordered, TryStreamExt},
};

use near_account_id::AccountId;
use near_kit::{BlockReference, CryptoHash, Finality, RpcClient, RpcError};
#[cfg(feature = "tracing")]
use tracing::{Span, field, instrument, record_all};

use crate::AuthorizationResolution;

/// RPC resolver for NEP-641 offchain authorizations.
#[derive(Debug, Clone)]
pub struct RpcResolver {
    client: RpcClient,
    chain_id: String,

    /// Block reference to resolve **all** authorizations against.
    at_block: BlockReference,

    /// Maximum allowed total number of sub-authorizations for a single top-level one.
    max_sub_auths: usize,
    /// Maximum allowed depth of sub-authorization branches.
    max_depth: usize,
    // state_inits: HashMap<AccountId, StateInit>,
}

impl RpcResolver {
    /// Create new verifier with given Near RPC client.
    #[must_use]
    pub async fn new(client: RpcClient) -> Result<Self, RpcError> {
        let status = client.status().await?;

        Ok(Self {
            client,
            chain_id: status.chain_id,
            // resolve against final block by default
            at_block: BlockReference::Finality(Finality::Final),
            // allow only top-level authorizations by default
            max_sub_auths: 0,
            max_depth: 0,
        })
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

    /// Set an upper limit for total number of sub-authorizations for a single top-level one.
    ///
    /// By default, this value is set to zero, so that only top-level authorizations are allowed
    /// and any sub-authorizations will fail. This is too concervative for real world use-cases,
    /// but used as a sane default to prevent from DoS attacks.
    ///
    /// Note that this doesn't change the [maximum depth](Self::with_max_depth) and it should be
    /// configured separately.
    #[must_use]
    #[inline]
    pub const fn with_max_sub_authorizations(mut self, n: usize) -> Self {
        self.max_sub_auths = n;
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
    /// This also raises the [maximum sub-authorizations](Self::with_max_sub_authorizations) limit
    /// to at least `max_depth`.
    #[must_use]
    #[inline]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        // this is a const version of: `self.max_pending = self.max_pending.max(self.max_depth)`
        if self.max_sub_auths < self.max_depth {
            self.max_sub_auths = self.max_depth;
        }
        self
    }

    /// Returns chain ID of underlying RPC client
    #[inline]
    pub const fn chain_id(&self) -> &str {
        self.chain_id.as_str()
    }

    /// Resolve a payload from top-level authorization according to NEP-641.
    ///
    /// This method recursively resolves given top-level authorization and all returned pending
    /// ones until no more authorizations are left, and returns a top-level authorized
    /// [payload](field@AuthorizationResolution::payload). If at least one authorization resolution
    /// fails or any [pending sub-authorization](crate::PendingAuthorization) resolves into a
    /// payload that doesn't match the [expected](field@crate::PendingAuthorization::expect) one,
    /// then the whole resolution procedure is immediately aborted and an error is returned.
    ///
    /// # Full-access keys
    ///
    /// For each authorization, the resolver attempts both
    /// [`AccessKeyAuthorization`](crate::access_keys::AccessKeyAuthorization) resolution and the
    /// [`w_resolve_auth()`](crate::AuthResolver::w_resolve_auth) view-method. A successfully
    /// resolved access-key authorization is verified according to
    /// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md), authorizes the signed
    /// payload without pending sub-authorizations, and takes precedence over contract resolution.
    ///
    // TODO: # Not yet initialized accounts
    /// # Block reference
    ///
    /// **All** authorizations are resolved against the same block hash to enforce consistent
    /// state between async RPC view-calls. By default, this method will fetch the `Final` block
    /// hash during top-level authorization resolution and resolve all pending ones against it.
    /// Be aware that RPC endpoint MAY be _out-of-sync_ and lag behind the tip of the network.
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
    /// See [`.with_max_sub_authorizations()`](Self::with_max_sub_authorizations) and
    /// [`.with_max_depth()`](Self::with_max_depth) to set your custom limits.
    pub async fn resolve_auth(
        &self,
        account_id: impl Into<AccountId>,
        authorization: String,
    ) -> Result<String, ResolveError> {
        // resolve top-level authorization first
        let ResolvedAuthorization {
            mut account_id,
            mut path,
            res:
                Resolved {
                    res:
                        AuthorizationResolution {
                            payload, // resolved top-level payload
                            mut pending,
                        },
                    block_hash, // resolved block hash
                    ..
                },
            #[cfg(feature = "tracing")]
            mut span,
        } = self
            .resolve(
                account_id.into(),
                vec![], // path is empty for top-level authorization
                authorization,
                None, // no expected payload for top-level authorization
                self.at_block.clone(),
                #[cfg(feature = "tracing")]
                Span::current(),
            )
            .await?;

        // keep track of total number of pending sub-authorizations to be resolved
        let mut sub_count: usize = 0;
        // a pool of futures to resolve all pending sub-authorizations concurrently
        let mut in_flight = FuturesUnordered::new();

        loop {
            // check if max depth limit will be exceeded for pending sub-authorizations, if any
            if !pending.is_empty() && path.len() >= self.max_depth {
                return Err(ResolveErrorKind::MaxDepthExceeded(self.max_depth).at(account_id, path));
            }

            sub_count = sub_count.saturating_add(pending.len());
            // check if adding new pending sub-authorizations wouldn't exceed max pending limit
            if sub_count > self.max_sub_auths {
                return Err(
                    ResolveErrorKind::TooManySubAuthorizations(self.max_sub_auths)
                        .at(account_id, path),
                );
            }

            // append parent resolver ID to the path for pending sub-authorizations, if any
            path.push(account_id);

            // add pending sub-authorizations to the in-flight pool
            in_flight.extend(pending.into_iter().map(|pending| {
                self.resolve(
                    pending.account_id,
                    path.clone(), // propagate extended path to all sub-resolvers
                    pending.authorization,
                    Some(pending.expect), // check that returned payload matches the expected one
                    block_hash.into(), // resolve pending sub-authorizations at the same block hash
                    #[cfg(feature = "tracing")]
                    span.clone(),
                )
            }));

            // wait until a pending sub-authorization resolves, if any
            let Some(resolved) = in_flight.try_next().await? else {
                // no more sub-authorizations left, return the top-level resolved payload
                return Ok(payload);
            };

            // overwrite variables from the resolved sub-authorization
            ResolvedAuthorization {
                account_id,
                path,
                res: Resolved {
                    res: AuthorizationResolution { pending, .. },
                    ..
                },
                #[cfg(feature = "tracing")]
                span,
            } = resolved;
        }
    }

    /// Resolve a single authorization and check that returned payload matches the expected one,
    /// if set
    #[cfg_attr(feature = "tracing", instrument(
        name = "resolve_auth",
        parent = parent_span,
        skip_all,
        fields(
            chain.id = self.chain_id,
            %account_id,
            depth = path.len(),
            at_block.finality = if let BlockReference::Finality(f) = block { Some(f.as_str()) } else { None }.map(field::display),
            at_block.hash = if let BlockReference::Hash(h) = block { Some(h) } else { None }.map(field::display),
            at_block.height = if let BlockReference::Height(h) = block { Some(h) } else { None },
        ),
    ))]
    async fn resolve(
        &self,
        account_id: AccountId,
        path: Vec<AccountId>,
        authorization: String,
        expect: Option<String>,
        block: BlockReference,
        #[cfg(feature = "tracing")] parent_span: Span,
    ) -> Result<ResolvedAuthorization, ResolveError> {
        #[cfg(feature = "tracing")]
        let span = Span::current();

        // try to resolve via both FullAccessKey and `w_resolve_auth()` contract view-method
        let (access_key, contract) = join!(
            self.resolve_access_key(&account_id, &path, &authorization, block.clone()),
            self.resolve_contract(&account_id, &path, &authorization, block.clone()),
            // TODO: add optional support for fallback to Intents verifier contract as a resolver
        );

        let res = match (access_key, contract) {
            // if both failed, but access key authorization at least deserialized successfully,
            // then propagate the error coming from it
            (res @ Err(ResolveErrorKind::AccessKey(_)), Err(_)) => res,
            // successfull resolution via FullAccessKey takes precedence over the contract, since it:
            // * has the same full control over the account
            // * authorizes a payload without pending sub-authorizations
            // * can be used as a last resort for a broken contract
            (access_key, contract) => access_key.or(contract),
        };

        // same as `res.map_err(|e| e.at(...))?` but avoids cloning
        let res = match res {
            Ok(res) => res,
            Err(err) => return Err(err.at(account_id, path)),
        };

        // check block returned by RPC, just in case
        {
            let mismatch = match block {
                BlockReference::Hash(hash) => res.block_hash != hash,
                BlockReference::Height(height) => res.block_height != height,
                _ => false,
            };
            if mismatch {
                return Err(ResolveErrorKind::from(RpcError::InvalidResponse(
                    "returned block doesn't match the requested one".to_string(),
                ))
                .at(account_id, path));
            }
        }
        // update `at_block.hash` and `at_block.height` with resolved block
        #[cfg(feature = "tracing")]
        record_all!(span, at_block.hash = %res.block_hash, at_block.height = res.block_height);

        // check if resolved payload matches the one expected by the parent resolver
        if let Some(expected) = expect
            && expected != res.res.payload
        {
            return Err(ResolveErrorKind::InvalidPayload {
                payload: res.res.payload,
                expected,
            }
            .at(account_id, path));
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            payload = res.res.payload,
            sub_authorizations = res.res.pending.len(),
            "authorization resolved"
        );

        Ok(ResolvedAuthorization {
            account_id,
            path,
            res,
            #[cfg(feature = "tracing")]
            span,
        })
    }
}

/// A single resolved authorization
struct ResolvedAuthorization {
    /// Account ID which resolved this authorization
    account_id: AccountId,

    /// Path at the time of resolution. Empty path means it was a top-level authorization.
    path: Vec<AccountId>,

    /// Resolved authorization
    res: Resolved,

    /// Span where this authorization was resolved
    #[cfg(feature = "tracing")]
    span: Span,
}

struct Resolved {
    /// Authorization resolution
    res: AuthorizationResolution,

    /// Resolved block hash
    block_hash: CryptoHash,

    /// Resolved block height
    block_height: u64,
}
