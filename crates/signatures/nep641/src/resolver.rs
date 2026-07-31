use std::collections::{HashMap, HashSet};

use futures::stream::{FuturesUnordered, TryStreamExt};
use near_account_id::AccountId;
use near_kit::{BlockReference, CryptoHash, Finality, Near};
#[cfg(feature = "tracing")]
use tracing::{Span, field, instrument, record_all};

use crate::{OffchainMessage, Proof, client::WResolveAuthArgs};

/// Verifier for NEP-641 [offchain messages](OffchainMessage).
#[derive(Debug, Clone)]
pub struct OffchainResolver {
    client: Near,
    at_block_hash: Option<CryptoHash>,
    max_pending: usize,
    // state_inits: HashMap<AccountId, StateInit>,
}

impl OffchainResolver {
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
            max_pending: usize::MAX,
        }
    }

    // TODO: uncomment when RPC adds "pre-init" support for view-calls
    // #[inline]
    // pub fn with_state_init(mut self, state_init: impl Into<StateInit>) -> Self {
    //     let state_init = state_init.into();
    //     self.state_inits.insert(state_init.derive_account_id(), state_init);
    //     self
    // }

    #[allow(clippy::doc_markdown)]
    /// Set an upper limit for maxumim number of pending sub-authorizations to resolve.
    ///
    /// A value of zero means that only top-level authorizations are allowed.
    ///
    /// By default, there is _no_ limit. It's recommended to set one to prevent from DoS attacks.
    #[must_use]
    #[inline]
    pub const fn with_max_pending(mut self, max_pending: usize) -> Self {
        self.max_pending = max_pending;
        self
    }

    /// Override block hash for [resolving](crate::contract::Nep641::w_resolve_auth)
    /// **all** autorizations.
    ///
    /// **All** authorizations are resolved against the same block hash to enforce
    /// consistent state between async RPC view-calls. By default,
    /// [`.resolve_auth()`](OffchainVerifier::resolve_auth) fetches the `Final` block
    /// hash first and then resolves all authorizations against it. This setting overrides
    /// it and allows to resolve authorizations against the chain state from the past.
    #[must_use]
    #[inline]
    pub fn at_block_hash(mut self, fixed_block_hash: impl Into<CryptoHash>) -> Self {
        self.at_block_hash = Some(fixed_block_hash.into());
        self
    }

    /// Resolve (i.e. verify) top-level authorization for given [offchain message](OffchainMessage)
    /// according to NEP-641.
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
        %msg.resolver_id,
        %msg.signer_id,
        %msg.chain_id,
        msg.hash = %bs58::encode(msg.hash()).into_string(),
        at_block.hash, // will be recorded after top-level resolve
    )))]
    // TODO: return signer_id? but this doesn't force the caller
    // to check the actual message being signed...
    pub async fn resolve_auth(
        &self,
        msg: OffchainMessage,
        proof: Proof,
    ) -> Result<(), ResolveError> {
        // TODO: maybe leave it for caller? intents.near
        if !msg.is_top_level() {
            return Err(ResolveError::NonTopLevel);
        }

        // a pool of futures to resolve authorizations concurrently
        let mut in_flight = FuturesUnordered::new();
        // keep track of already seen account IDs
        let mut seen = HashSet::new();

        // mark top-level resolver_id as already seen
        seen.insert(msg.resolver_id.clone());
        // if set, resolve top-level authorization at fixed block hash, or final otherwise
        in_flight.push(self.resolve_single(msg.clone(), proof, self.at_block_hash));

        // pinned block hash, will be populated from top-level `self.resolve_single()` above
        let mut at_block_hash: CryptoHash;

        // resolve until no more pending authorizations are left
        while let Some(resolved) = in_flight.try_next().await? {
            // populate fetched block hash from top-level `self.resolve_signed()`
            // TODO: a lagging RPC can resolve old Final block
            at_block_hash = resolved.block_hash;
            #[cfg(feature = "tracing")]
            record_all!(Span::current(), at_block.hash = %at_block_hash);

            for (resolver_id, proof) in resolved.pending {
                // TODO: it means that cycles automatically cancel each other out,
                // even if proofs are different. Is it ok?
                if !seen.insert(resolver_id.clone()) {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        "resolver_id {} has been already seen, skipping...",
                        resolver_id
                    );
                    continue;
                }
                // TODO: better tracing
                // TODO: cycles?

                // `seen` has top-level `resolver_id` already, so we need to subtract one
                if seen.len() - 1 > self.max_pending {
                    // prevent DoS attack in case of malicious contract(s)
                    // returns too many pending authorizations
                    return Err(ResolveError::TooManyAuthorizations(self.max_pending));
                }

                // resolve pending authorizations at the same block hash
                in_flight.push(self.resolve_single(
                    // override resolver for sub-authorization
                    msg.clone().with_resolver_id(resolver_id),
                    proof,
                    Some(at_block_hash),
                ));
            }
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(
        %msg.resolver_id,
        %msg.signer_id,
        %msg.chain_id,
        msg.hash = %bs58::encode(msg.hash()).into_string(),
        at_block.hash = at_block_hash.map(field::display),
    )))]
    async fn resolve_single(
        &self,
        msg: OffchainMessage,
        proof: Proof,
        at_block_hash: Option<CryptoHash>,
    ) -> Result<SingleResolved, ResolveError> {
        if msg.chain_id != self.client.chain_id().as_str() {
            return Err(ResolveError::InvalidChainId);
        }

        let res = self
            .client
            .rpc()
            .view_function(
                &msg.resolver_id,
                "w_resolve_auth",
                &serde_json::to_vec(&WResolveAuthArgs::from((&msg, proof.as_str())))
                    .expect("JSON: serialization failed"),
                at_block_hash.map_or(
                    // use final block hash by default
                    BlockReference::Finality(Finality::Final),
                    Into::into,
                ),
                // TODO: "pre-init" if we StateInit for this AccountId
                // self.state_inits.get(&msg.resolver_id),
            )
            // TODO: handle contract errors
            .await?;

        // if was set, make sure RPC returned same block hash
        if let Some(at_block_hash) = at_block_hash
            && at_block_hash != res.block_hash
        {
            // TODO: RPCs can be behind a load-balancer, so that they can return UnknownBlock error
            // TODO: maybe we need to retry with minimum-known block_height?
            return Err(
                near_kit::RpcError::InvalidResponse("block hash mismatch".to_string()).into(),
            );
        }

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

        Ok(SingleResolved {
            pending: res.json()?,
            block_hash: res.block_hash,
        })
    }
}

// TODO: rename?
struct SingleResolved {
    pending: HashMap<AccountId, Proof>,
    block_hash: CryptoHash,
}

/// An error returned by [`OffchainResolver`]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveError {
    #[error("the authorization is not top-level")]
    NonTopLevel,

    #[error("invalid chain_id")] // TODO
    InvalidChainId,

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
