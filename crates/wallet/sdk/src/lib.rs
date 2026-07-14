#[cfg(feature = "near-kit")]
pub mod client;
mod nonces;
mod signer;

pub use self::{nonces::*, signer::*};

pub use defuse_wallet::*;

use std::{
    collections::BTreeSet,
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};

use borsh::BorshSerialize;
use impl_tools::autoimpl;
use near_global_contracts::{GlobalContractId, StateInit, StateInitV1};
use rand::{make_rng, rngs::SmallRng};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, record_all};

/// `mainnet` chain id
pub const MAINNET: &str = "mainnet";

/// Builder for [`Wallet`]
#[must_use = "`.build()` the signer"]
#[derive(Debug)]
pub struct WalletBuilder {
    subwallet_id: u32,
    timeout: Duration,
    extensions: BTreeSet<AccountId>,
}

impl Default for WalletBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl WalletBuilder {
    /// Create a builder with default parameters.
    #[inline]
    pub const fn new() -> Self {
        Self {
            subwallet_id: DEFAULT_SUBWALLET_ID,
            timeout: DEFAULT_TIMEOUT,
            extensions: BTreeSet::new(),
        }
    }

    /// Set a custom `subwallet_id` instead of [default](DEFAULT_SUBWALLET_ID) one.
    /// This can be used to derive multiple wallet-contract instances
    /// from a single public key.
    #[inline]
    pub const fn subwallet_id(mut self, subwallet_id: u32) -> Self {
        self.subwallet_id = subwallet_id;
        self
    }

    /// Set a custom `timeout` (i.e. maximum validity for each nonce) instead
    /// of the [default](`DEFAULT_TIMEOUT`) one.
    ///
    /// NOTE: the longer the timeout, the more storage usage in highload environments.
    /// Setting a long timeout might result in locking large amounts of NEAR tokens for
    /// storage staking for `2 * timeout` time window.
    #[inline]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Pre-enable extensions with given account ids.
    #[inline]
    pub fn extensions(mut self, account_ids: impl IntoIterator<Item = AccountId>) -> Self {
        self.extensions.extend(account_ids);
        self
    }

    /// Derive and build a [`Wallet`] handle for a wallet instance by signer's public key
    /// and an id of (globally deployed) wallet contract code.
    ///
    /// NOTE: this itself does **not** create an account on NEAR. See
    /// [`.deterministic_state_init()`](Wallet::deterministic_state_init).
    pub fn build<SS, S>(self, code: impl Into<GlobalContractId>, signer: S) -> Wallet<SS, S>
    where
        SS: SignatureSchema<PublicKey: BorshSerialize>,
        S: WalletSigner<SS>,
    {
        let state_init = StateInit::V1(StateInitV1 {
            code: code.into(),
            data: State::new(signer.public_key())
                .subwallet_id(self.subwallet_id)
                .timeout(self.timeout)
                .extensions(self.extensions)
                .as_storage(),
        });

        Wallet {
            account_id: state_init.derive_account_id(),
            state_init,
            subwallet_id: self.subwallet_id,
            timeout: self.timeout,
            nonces: Arc::new(Mutex::new(ConcurrentNonces::new(make_rng()))),
            signer,
            _schema: PhantomData,
        }
    }
}

/// Signer handle to a wallet contract instance implementing a specific
/// [`SignatureSchema`].
#[autoimpl(Debug, Clone where S: trait)]
pub struct Wallet<SS: SignatureSchema, S: WalletSigner<SS>> {
    account_id: AccountId,
    state_init: StateInit,

    subwallet_id: u32,
    timeout: Duration,
    nonces: Arc<Mutex<ConcurrentNonces<SmallRng>>>,

    signer: S,
    _schema: PhantomData<SS>,
}

impl<SS, S> Wallet<SS, S>
where
    SS: SignatureSchema,
    S: WalletSigner<SS>,
{
    /// Shorthand for [`WalletBuilder::new()`].[`build()`](WalletBuilder::build).
    #[inline]
    pub fn new(code: impl Into<GlobalContractId>, signer: S) -> Self
    where
        SS::PublicKey: BorshSerialize,
    {
        WalletBuilder::new().build(code, signer)
    }

    /// Get derived account id for this wallet contract instance.
    ///
    /// NOTE: the account on NEAR might **not** exist yet and needs to be
    /// initialized first. See [`.deterministic_state_init()`](Self::deterministic_state_init)
    #[inline]
    pub const fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    /// Get initialization state for this wallet contract instance.
    ///
    /// A first transaction to the wallet's account needs to include
    /// [`.deterministic_state_init()`](Wallet::deterministic_state_init) action in order
    /// to initialize the contract before calling methods on it. Relayers should have a
    /// support for passing (optional) state init along signed requests.
    #[inline]
    pub const fn deterministic_state_init(&self) -> &StateInit {
        &self.state_init
    }

    /// Get `subwallet_id` of this wallet contract instance.
    #[inline]
    pub const fn subwallet_id(&self) -> u32 {
        self.subwallet_id
    }

    /// Get `timeout` (i.e. fixed maximum validity for each nonce) of this wallet contract
    /// instance.
    #[inline]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get a reference to the underlying [signer](WalletSigner)
    #[inline]
    pub const fn signer(&self) -> &S {
        &self.signer
    }

    /// Get [signer](Self::signer)'s public key
    #[inline]
    pub fn public_key(&self) -> SS::PublicKey {
        self.signer().public_key()
    }

    /// Wrap given request in a [`RequestMessage`] for given chain id and sign it.
    ///
    /// # Chain Id
    ///
    /// A single signer can control wallet contract instances with same account id on
    /// different chains. So, each signed message needs to include id of a chain where
    /// it's intended to be executed on.
    #[allow(clippy::future_not_send)]
    #[cfg_attr(feature = "tracing", instrument(level = Level::DEBUG, skip_all, fields(
        msg.chain_id,
        msg.signer_id,
        msg.nonce,
        msg.created_at,
        msg.timeout_secs,
        msg.hash
    )))]
    pub async fn sign(
        &self,
        request: Request,
        chain_id: impl Into<String>,
    ) -> Result<(RequestMessage, Proof), S::Error> {
        let msg = self.wrap_request_msg(request, chain_id);

        #[cfg(feature = "tracing")]
        record_all!(
            tracing::Span::current(),
            msg.chain_id,
            %msg.signer_id,
            msg.nonce,
            %msg.created_at,
            msg.timeout_secs = msg.timeout.as_secs(),
            msg.hash = %bs58::encode(msg.hash()).into_string(),
        );

        let proof = self.signer.sign_request_msg(&msg).await?;

        debug_assert!(
            SS::verify(&self.signer.public_key(), &msg, &proof),
            "signer produced invalid signature",
        );

        Ok((msg, proof))
    }

    /// Wraps [`Request`] in [`RequestMessage`] for signing
    #[must_use = "`.sign()` the wrapped request"]
    #[inline]
    fn wrap_request_msg(&self, request: Request, chain_id: impl Into<String>) -> RequestMessage {
        RequestMessage {
            chain_id: chain_id.into(),
            signer_id: self.account_id().clone(),
            nonce: self.nonces.lock().unwrap().next(),
            // set `created_at` slightly before the actual time of signing,
            // so it doesn't fail on-chain if arrives too fast.
            created_at: Timestamp::now() - self.optimal_lag(),
            timeout: self.timeout(),
            request,
        }
    }

    /// Returns an optimal lag for `created_at`, so it doesn't fail on-chain.
    #[inline]
    fn optimal_lag(&self) -> Duration {
        Duration::from_mins(1).min(self.timeout() / 5)
    }

    /// Reseed the [nonces](ConcurrentNonces) and invalidate the current block.
    /// Use it in case of a collision.
    #[inline]
    pub fn reseed_nonces(&self) {
        *self.nonces.lock().unwrap() = ConcurrentNonces::new(make_rng());
    }
}
