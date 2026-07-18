#[cfg(feature = "near-kit")]
pub mod client;
#[cfg(feature = "mpc")]
use defuse_mpc_signer as mpc;
use defuse_wallet::actions::FunctionCall;
mod nonces;
#[cfg(feature = "relayer")]
pub mod relayer;
mod signer;

pub use self::signer::*;

pub use defuse_wallet::*;

use std::{
    collections::BTreeSet,
    error::Error as StdError,
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};

use borsh::BorshSerialize;
use impl_tools::autoimpl;
use rand::{make_rng, rngs::SmallRng};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, record_all};

use crate::{client::WExecuteExtensionArgs, nonces::ConcurrentNonces};

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
    pub fn build<S, SS>(self, code: impl Into<GlobalContractId>, signer: SS) -> Wallet<S>
    where
        S: SignatureSchema<PublicKey: BorshSerialize>,
        SS: WalletSigner<S, Error: Into<Box<dyn StdError>>> + 'static,
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
            chain_id: MAINNET.to_string(),
            signer: signer.arced(),
            _schema: PhantomData,
            as_extension_chain: Vec::new(),
            #[cfg(feature = "near-kit")]
            client: None,
            #[cfg(feature = "relayer")]
            relayer: None,
            #[cfg(feature = "mpc")]
            mpc_contract_id: None,
        }
    }
}

/// Signer handle to a wallet contract instance implementing a specific
/// [`SignatureSchema`].
// TODO: make it clonable
#[autoimpl(Clone)]
pub struct Wallet<S: SignatureSchema> {
    account_id: AccountId,
    state_init: StateInit,

    subwallet_id: u32,
    timeout: Duration,
    nonces: Arc<Mutex<ConcurrentNonces<SmallRng>>>,

    chain_id: ChainId,

    signer: ArcWalletSigner<S>,
    // `fn() -> S` implements Send + Sync unconditionally
    _schema: PhantomData<fn() -> S>,

    as_extension_chain: Vec<AccountId>,

    #[cfg(feature = "near-kit")]
    client: Option<near_kit::Near>,

    #[cfg(feature = "relayer")]
    relayer: Option<relayer::ArcWalletRelayer>,

    #[cfg(feature = "mpc")]
    mpc_contract_id: Option<AccountId>,
}

impl<S> Wallet<S>
where
    S: SignatureSchema,
{
    #[allow(clippy::doc_link_code)]
    /// Shorthand for [`WalletBuilder::new()`](WalletBuilder::new)[`.build()`](WalletBuilder::build).
    #[inline]
    pub fn new<SS>(code: impl Into<GlobalContractId>, signer: SS) -> Self
    where
        S::PublicKey: BorshSerialize,
        SS: WalletSigner<S, Error: Into<Box<dyn StdError>>> + 'static,
    {
        WalletBuilder::new().build(code, signer)
    }

    // TODO: rename: with_chain_id?
    /// Set a custom [`chain_id`](RequestMessage::chain_id) for [`.sign()`](Self::sign)
    /// instead of a [default](MAINNET) one.
    #[must_use]
    #[inline]
    pub fn with_chain_id(mut self, chain_id: impl Into<ChainId>) -> Self {
        let new_chain_id = chain_id.into();
        if self.chain_id != new_chain_id {
            // contracts on different chains keep track of their own nonces
            self.reseed_nonces();
        }
        self.chain_id = new_chain_id;
        self
    }

    // TODO: flush_extensions
    #[inline]
    pub fn as_extension_of(mut self, account_id: impl Into<AccountId>) -> Self {
        self.as_extension_chain.push(account_id.into());
        self
    }

    // pub fn pop_extension

    #[cfg(feature = "near-kit")]
    #[must_use]
    #[inline]
    pub fn with_client(mut self, client: near_kit::Near) -> Self {
        // TODO: are we sure?
        self = self.with_chain_id(client.chain_id().as_str());
        self.client = Some(client);
        self
    }

    #[cfg(feature = "relayer")]
    #[must_use]
    #[inline]
    pub fn with_relayer<R>(mut self, relayer: R) -> Self
    where
        R: relayer::WalletRelayer<Error: Into<Box<dyn StdError>>> + 'static,
    {
        self.relayer = Some(relayer.arced());
        self
    }

    #[cfg(feature = "mpc")]
    #[must_use]
    #[inline]
    pub fn with_mpc_contract_id(mut self, mpc_contract_id: impl Into<AccountId>) -> Self {
        self.mpc_contract_id = Some(mpc_contract_id.into());
        self
    }

    #[inline]
    pub const fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    // TODO: docs: effective/real
    /// Get derived account id for this wallet contract instance.
    ///
    /// NOTE: the account on NEAR might **not** exist yet and needs to be
    /// initialized first. See [`.deterministic_state_init()`](Self::deterministic_state_init)
    #[inline]
    pub const fn account_id(&self) -> &AccountId {
        if let Some(last_extension_id) = self.as_extension_chain.as_slice().last() {
            return last_extension_id;
        }

        self.real_account_id()
    }

    #[inline]
    pub const fn real_account_id(&self) -> &AccountId {
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

    /// Get [`subwallet_id`](field@State::subwallet_id) of this wallet contract instance.
    #[inline]
    pub const fn subwallet_id(&self) -> u32 {
        self.subwallet_id
    }

    /// Get [signer](Self::signer)'s public key
    #[inline]
    pub fn public_key(&self) -> S::PublicKey {
        self.signer.public_key()
    }

    /// Get `timeout` (i.e. fixed maximum validity for each nonce) of this wallet contract
    /// instance.
    #[inline]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[cfg(feature = "near-kit")]
    #[inline]
    fn try_client(&self) -> Option<near_kit::Near> {
        self.client.clone()
    }

    #[cfg(feature = "near-kit")]
    #[track_caller]
    #[inline]
    fn client(&self) -> near_kit::Near {
        self.try_client().expect("client is not set")
    }

    #[cfg(feature = "near-kit")]
    #[inline]
    pub fn is_signature_allowed(&self) -> near_kit::ViewCall<bool> {
        use crate::client::WalletContract;

        self.client()
            // TODO: are we sure: real_account_id?
            .contract::<WalletContract>(self.real_account_id())
            .w_is_signature_allowed()
    }

    #[cfg(feature = "near-kit")]
    #[inline]
    pub fn is_extension_enabled(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> near_kit::ViewCall<bool> {
        use crate::client::WalletContract;

        self.client()
            .contract::<WalletContract>(self.account_id())
            .w_is_extension_enabled(account_id.as_ref().into())
    }

    #[cfg(feature = "near-kit")]
    #[inline]
    pub fn extensions(&self) -> near_kit::ViewCall<BTreeSet<AccountId>> {
        use crate::client::WalletContract;

        self.client()
            .contract::<WalletContract>(self.account_id())
            .w_extensions()
    }

    #[cfg(feature = "relayer")]
    #[inline]
    fn try_relayer(&self) -> Option<&dyn relayer::DynWalletRelayer> {
        self.relayer.as_deref()
    }

    #[cfg(feature = "relayer")]
    #[track_caller]
    #[inline]
    fn relayer(&self) -> &dyn relayer::DynWalletRelayer {
        // TODO: better panic
        self.try_relayer().expect("relayer is not set")
    }

    /// Wrap given request in a [`RequestMessage`] and sign it.
    ///
    /// # Chain Id
    ///
    /// A single signer can control wallet contract instances with same account id on
    /// different chains. So, each signed message needs to include id of a chain where
    /// it's intended to be executed on.
    #[cfg_attr(feature = "tracing", instrument(level = Level::DEBUG, skip_all, fields(
        msg.chain_id = &self.chain_id,
        msg.signer_id = %self.account_id(),
        msg.nonce,
        msg.created_at,
        msg.timeout_secs,
        msg.hash
    )))]
    // TODO: rename: sign_request_msg
    pub async fn sign(
        &self,
        request: impl Into<Request>,
    ) -> Result<(RequestMessage, Proof), Error> {
        let msg = self.wrap_request_msg(request);

        #[cfg(feature = "tracing")]
        record_all!(
            tracing::Span::current(),
            msg.nonce,
            %msg.created_at,
            msg.timeout_secs = msg.timeout.as_secs(),
            msg.hash = %bs58::encode(msg.hash()).into_string(),
        );

        let proof = self
            .signer
            .sign_request_msg(&msg)
            .await
            .map_err(Error::Signer)?;

        // TODO: emit event with msg hash

        debug_assert!(
            S::verify(&self.signer.public_key(), &msg, &proof),
            "signer produced invalid signature",
        );

        Ok((msg, proof))
    }

    /// Wraps [`Request`] in [`RequestMessage`] for signing
    #[must_use = "`.sign()` the wrapped request"]
    #[inline]
    fn wrap_request_msg(&self, request: impl Into<Request>) -> RequestMessage {
        RequestMessage {
            chain_id: self.chain_id.clone(),
            signer_id: self.real_account_id().clone(),
            nonce: self.nonces.lock().unwrap().next(),
            // set `created_at` slightly before the actual time of signing,
            // so it doesn't fail on-chain if arrives too fast.
            created_at: Timestamp::now() - self.optimal_lag(),
            timeout: self.timeout(),
            // TODO: explain in comment
            request: self
                .as_extension_chain
                .iter()
                .rfold(request.into(), |request, extension| {
                    NearPromise::new(extension)
                        .function_call(
                            FunctionCall::name("w_execute_extension")
                                .attach_deposit(NearToken::from_yoctonear(1))
                                // TODO: gas
                                .args_json(WExecuteExtensionArgs::from(request)),
                        )
                        .into()
                }),
        }
    }

    /// Returns an optimal lag for `created_at`, so it doesn't fail on-chain
    /// if arrives too early.
    #[inline]
    fn optimal_lag(&self) -> Duration {
        Duration::from_mins(1).min(self.timeout() / 5)
    }

    /// TODO: docs: relayer must be set
    #[cfg(feature = "relayer")]
    pub async fn sign_and_relay(
        &self,
        request: impl Into<Request>,
    ) -> Result<defuse_near_sender::SentTransaction, Error> {
        use crate::relayer::WalletRelayer;
        // check before signing if relayer is set
        let relayer = self.relayer();

        let (msg, proof) = self.sign(request).await?;

        relayer
            .relay_signed_msg(msg, proof)
            .await
            .map_err(Error::Relayer)
    }

    #[cfg(feature = "mpc")]
    pub async fn mpc_signer<C>(
        &self,
        domain_id: u64,
    ) -> Result<mpc::MpcOnChainSigner<C, Self>, mpc::Error<Error>>
    where
        C: mpc::OnChainNearMpcCurve,
    {
        mpc::MpcOnChainSigner::new(
            self.clone(), // TODO: or require self?
            self.mpc_contract_id
                .clone()
                .expect("mpc_contract_id is not set"),
            domain_id,
            self.client(),
        )
        .await
    }

    /// Reseed the [nonces](ConcurrentNonces) and invalidate the current block.
    /// Use it in case of a collision.
    #[inline]
    pub fn reseed_nonces(&self) {
        *self.nonces.lock().unwrap() = ConcurrentNonces::new(make_rng());
    }
}

impl<S: SignatureSchema> AsRef<AccountIdRef> for Wallet<S> {
    // TODO: docs: returns effective
    #[inline]
    fn as_ref(&self) -> &AccountIdRef {
        self.account_id()
    }
}

// TODO: non_exhaustive?
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("relayer: {0}")]
    Relayer(Box<dyn StdError>),
    #[error("signer: {0}")]
    Signer(Box<dyn StdError>),
}
