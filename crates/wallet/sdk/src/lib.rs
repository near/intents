#[cfg(feature = "near-kit")]
pub mod client;
#[cfg(feature = "mpc")]
pub use defuse_mpc_signer as mpc;
use defuse_near_sender::{NearSender, SentTransaction};
use defuse_wallet::actions::NearAction;
mod nonces;
pub mod relayer;
mod signer;

pub use self::signer::*;

pub use defuse_wallet::*;

use std::{
    borrow::Cow,
    collections::BTreeSet,
    error::Error as StdError,
    mem,
    sync::{Arc, Mutex},
    time::Duration,
};

use borsh::BorshSerialize;
use impl_tools::autoimpl;
use rand::{make_rng, rngs::SmallRng};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, record_all};

use crate::{
    actions::FunctionCall,
    client::WExecuteExtensionArgs,
    nonces::ConcurrentNonces,
    relayer::{DynWalletRelayer, WalletRelayRequest, WalletRelayer},
};

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
    /// and an ID of (globally deployed) contract code for wallet variant implementing
    /// this [`SignatureSchema`].
    ///
    /// NOTE: this itself does **not** create an account on NEAR. See
    /// [`.deterministic_state_init()`](Wallet::deterministic_state_init).
    pub fn build<S, SS>(self, code: impl Into<GlobalContractId>, signer: SS) -> Wallet<S>
    where
        S: SignatureSchema,
        S::PublicKey: BorshSerialize,
        SS: WalletSigner<S> + 'static,
        SS::Error: Into<Box<dyn StdError + Send + Sync>>,
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
            initialized: false,
            timeout: self.timeout,
            nonces: Arc::new(Mutex::new(ConcurrentNonces::new(make_rng()))),
            chain_id: MAINNET.to_string(),
            signer: Arc::new(signer),
            as_extension_chain: Vec::new(),
            #[cfg(feature = "near-kit")]
            client: None,
            relayer: None,
            #[cfg(feature = "mpc")]
            mpc_contract_id: Some(mpc::MAINNET_MPC_CONTRACT_ID.to_owned()),
        }
    }
}

/// Handle to a wallet contract implementing a specific [`SignatureSchema`].
///
/// # Examples
///
/// ```rust
/// use defuse_wallet_sdk::Wallet;
/// # use defuse_wallet_sdk::GlobalContractId;
/// use defuse_wallet_ed25519::{
///     WalletEd25519, WalletEd25519Signer,
///     crypto::ed25519::ed25519_dalek,
/// };
/// use rand::{rngs::SysRng, rand_core::UnwrapErr};
/// # const WALLET_ED25519_GLOBAL_CONTRACT_ID: GlobalContractId =
/// #     GlobalContractId::CodeHash([0u8; 32]);
///
/// // 1. Generate keypair
/// let signer = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
///
/// // 2. Build wallet for a specific signature schema
/// let wallet = Wallet::<WalletEd25519>::new(
///     WALLET_ED25519_GLOBAL_CONTRACT_ID,
///     WalletEd25519Signer(signer),
/// );
///
/// // 3. Derive account ID
/// println!("wallet: {}", wallet.account_id());
/// ```
#[autoimpl(Clone)]
pub struct Wallet<S: SignatureSchema> {
    state_init: StateInit,
    account_id: AccountId,
    initialized: bool,

    chain_id: ChainId,
    timeout: Duration,
    nonces: Arc<Mutex<ConcurrentNonces<SmallRng>>>,
    as_extension_chain: Vec<AccountId>,

    signer: Arc<dyn DynWalletSigner<S>>,

    #[cfg(feature = "near-kit")]
    client: Option<near_kit::Near>,

    relayer: Option<Arc<dyn DynWalletRelayer>>,

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
        SS: WalletSigner<S> + 'static,
        SS::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        WalletBuilder::new().build(code, signer)
    }

    /// Set a custom [`chain_id`](RequestMessage::chain_id) for [signing](Self::sign)
    /// requests instead of a [default](MAINNET) one.
    ///
    /// This doesn't change the [account ID](Self::account_id) of the wallet:
    /// a single signer can control multiple wallet contract instances with
    /// the same account ID on different chains by setting
    /// [`chain_id`](field@RequestMessage::chain_id) field in signed requests.
    ///
    /// This resets previously set [MPC contract ID](Self::with_mpc_contract_id)
    /// unless the chain ID didn't change.
    #[must_use]
    #[inline]
    pub fn with_chain_id(mut self, chain_id: impl Into<ChainId>) -> Self {
        let old_chain_id = mem::replace(&mut self.chain_id, chain_id.into());
        if self.chain_id != old_chain_id {
            // same wallet account ID on different chain might not have been
            // initialized yet
            self.initialized = false;
            // same wallet instances on different chains keep track of their own nonces
            self.reseed_nonces();

            #[cfg(feature = "mpc")]
            {
                // reset MPC contract ID
                self.mpc_contract_id =
                    (self.chain_id == MAINNET).then(|| mpc::MAINNET_MPC_CONTRACT_ID.to_owned());
            }
        }
        self
    }

    /// Configure Near client for this wallet.
    ///
    /// This also [resets chain ID](Self::with_chain_id) with client's one.
    #[cfg(feature = "near-kit")]
    #[must_use]
    #[inline]
    pub fn with_client(mut self, client: near_kit::Near) -> Self {
        self = self.with_chain_id(client.chain_id().as_str());
        self.client = Some(client);
        self
    }

    /// Configure [relayer](WalletRelayer) for this wallet.
    ///
    /// This is currently required for [`.sign_and_send()`](Self::sign_and_send) to work.
    #[must_use]
    #[inline]
    pub fn with_relayer<R>(mut self, relayer: R) -> Self
    where
        R: WalletRelayer + 'static,
        R::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        self.relayer = Some(Arc::new(relayer));
        self
    }

    /// Configure MPC contract ID for this wallet.
    /// See [`.mpc_signer()`](Self::mpc_signer) method.
    #[cfg(feature = "mpc")]
    #[must_use]
    #[inline]
    pub fn with_mpc_contract_id(mut self, mpc_contract_id: impl Into<AccountId>) -> Self {
        self.mpc_contract_id = Some(mpc_contract_id.into());
        self
    }

    /// Mark wallet's [real account ID](Self::real_account_id) as already
    /// initilized, so that future requests won't include unnecessary
    /// [state init](Self::deterministic_state_init).
    #[must_use]
    #[inline]
    pub const fn initialized(mut self) -> Self {
        self.initialized = true;
        self
    }

    /// Add another wallet to the extension chain, making it an
    /// [effective account id](Self::account_id), and act on behalf of it.
    ///
    /// This will automatically wrap all future [signed](Self::sign) requests to funnel them into
    /// [`master_id::w_execute_extension()`](crate::contract::Wallet::w_execute_extension)
    /// method, so that target [internal operations](field@Request::internal) are
    /// applied on the master wallet and created
    /// [external promises](field@Request::external) will have `master_id` as their
    /// predecessor ID.
    ///
    /// The master wallet SHOULD have **current** [effective account id](Self::account_id)
    /// already [added as an extension](crate::WalletOp::AddExtension).
    /// Otherwise, transactions will fail on-chain.
    ///
    /// # Panics
    ///
    /// This method panics if given account ID is [real account ID](Self::real_account_id)
    /// of this wallet or already in the extension chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_wallet_sdk::{Wallet, AccountIdRef};
    /// # use defuse_wallet_ed25519::{
    /// #   WalletEd25519, WalletEd25519Signer,
    /// #   crypto::ed25519::ed25519_dalek::SigningKey,
    /// # };
    /// # const SUBMASTER_WALLET_ID: &AccountIdRef = AccountIdRef::new_or_panic("sub.master");
    /// # const MASTER_WALLET_ID: &AccountIdRef = AccountIdRef::new_or_panic("master");
    /// # let wallet = Wallet::<WalletEd25519>::new(
    /// #     [0u8; 32],
    /// #     WalletEd25519Signer(SigningKey::from_bytes(&[0u8; 32])),
    /// # );
    /// // wallet -> submaster
    /// let as_sub_master = wallet.as_extension_of(SUBMASTER_WALLET_ID);
    /// assert_eq!(as_sub_master.account_id(), SUBMASTER_WALLET_ID);
    ///
    /// // wallet -> submaster -> master
    /// let as_master = as_sub_master.as_extension_of(MASTER_WALLET_ID);
    /// assert_eq!(as_master.account_id(), MASTER_WALLET_ID);
    /// ```
    #[must_use]
    #[track_caller]
    #[inline]
    pub fn as_extension_of(mut self, account_id: impl Into<AccountId>) -> Self {
        let account_id = account_id.into();

        assert!(
            account_id != *self.real_account_id() && !self.as_extension_chain.contains(&account_id),
            "extension cycle detected",
        );

        self.as_extension_chain.push(account_id);
        debug_assert_eq!(self.account_id(), self.as_extension_chain.last().unwrap());
        self
    }

    /// Reset the [extension chain](Self::as_extension_of) and act on behalf
    /// of the [real account ID](Self::real_account_id).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_wallet_sdk::{Wallet, AccountIdRef};
    /// # use defuse_wallet_ed25519::{
    /// #   WalletEd25519, WalletEd25519Signer,
    /// #   crypto::ed25519::ed25519_dalek::SigningKey,
    /// # };
    /// # const MASTER_WALLET_ID: &AccountIdRef = AccountIdRef::new_or_panic("master");
    /// # let wallet = Wallet::<WalletEd25519>::new(
    /// #     [0u8; 32],
    /// #     WalletEd25519Signer(SigningKey::from_bytes(&[0u8; 32])),
    /// # );
    /// let as_master = wallet.as_extension_of(MASTER_WALLET_ID);
    /// assert_eq!(as_master.account_id(), MASTER_WALLET_ID);
    ///
    /// let as_self = as_master.as_self();
    /// assert_eq!(as_self.account_id(), as_self.real_account_id());
    /// ```
    #[must_use]
    #[inline]
    pub fn as_self(mut self) -> Self {
        self.as_extension_chain.clear();
        debug_assert_eq!(self.account_id(), self.real_account_id());
        self
    }

    /// Returns currently [configured](Self::with_chain_id) chain ID for [signing](Self::sign)
    /// requests.
    #[inline]
    pub const fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    /// Get an _effective_ account ID which this wallet acts on behalf of.
    ///
    /// This is the last account ID from the currently configured
    /// [extension chain](Self::as_extension_of) or
    /// [`.real_account_id()`](Self::real_account_id) otherwise.
    #[inline]
    pub const fn account_id(&self) -> &AccountId {
        if let Some(last_extension_id) = self.as_extension_chain.as_slice().last() {
            return last_extension_id;
        }
        self.real_account_id()
    }

    /// Returns _real_ account ID of this wallet instance.
    ///
    /// NOTE: the account on NEAR might **not** exist yet and needs to be
    /// initialized first. See [`.deterministic_state_init()`](Self::deterministic_state_init)
    #[inline]
    pub const fn real_account_id(&self) -> &AccountId {
        &self.account_id
    }

    /// Get initialization state for [real account ID](Self::real_account_id) of this wallet.
    ///
    /// A first transaction to the wallet's [real account id](Self::real_account_id)
    /// needs to include [`.deterministic_state_init()`](Wallet::deterministic_state_init)
    /// action in order to initialize the contract before calling methods on it.
    #[inline]
    pub const fn deterministic_state_init(&self) -> &StateInit {
        &self.state_init
    }

    /// Get signer's public key
    #[inline]
    pub fn public_key(&self) -> S::PublicKey {
        self.signer.public_key()
    }

    /// Get `timeout`, i.e. fixed maximum validity for each nonce in signed
    /// requests
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
        self.try_client()
            .expect("client was not configured, use `with_client()` to set one")
    }

    #[inline]
    fn try_relayer(&self) -> Option<&dyn DynWalletRelayer> {
        self.relayer.as_deref()
    }

    #[track_caller]
    #[inline]
    fn relayer(&self) -> &dyn DynWalletRelayer {
        self.try_relayer()
            .expect("relayer was not configured, use `with_relayer()` to set one")
    }

    /// Sign on-chain request to be executed on behalf of
    /// [effective account ID](Self::account_id).
    ///
    /// The returned [`RequestMessage`] along with the proof should be delivered to
    /// [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed) method of
    /// [real account ID](Self::real_account_id) of this wallet.
    ///
    /// NOTE: The wallet account itself might **not** be initialized yet. See
    /// [`.deterministic_state_init()`](Wallet::deterministic_state_init).
    #[cfg_attr(feature = "tracing", instrument(level = Level::DEBUG, skip_all, fields(
        msg.chain_id = &self.chain_id,
        msg.signer_id = %self.account_id(),
        msg.nonce,
        msg.created_at,
        msg.timeout_secs,
        msg.hash
    )))]
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
            .sign_wallet_msg(&msg)
            .await
            .map_err(Error::Signer)?;

        debug_assert!(
            S::verify_request_msg(&self.signer.public_key(), &msg, &proof),
            "signer produced invalid signature",
        );

        Ok((msg, proof))
    }

    // TODO: sign offchain msg

    /// Wraps [`Request`] in [`RequestMessage`] for signing
    #[must_use = "`.sign()` the wrapped request"]
    #[inline]
    fn wrap_request_msg(&self, request: impl Into<Request>) -> RequestMessage {
        RequestMessage {
            pay_for_gas: false, // TODO: add support for External Contract Calls
            chain_id: self.chain_id.clone(),
            signer_id: self.real_account_id().clone(),
            nonce: self.nonces.lock().unwrap().next(),
            // Set `created_at` slightly before the actual time of signing,
            // so it doesn't fail on-chain if arrives too fast.
            created_at: Timestamp::now() - self.optimal_lag(),
            timeout: self.timeout(),
            // Recursively wrap request as `w_execute_extension()` FunctionCall
            // for each extension in the chain (starting from the last one)
            request: self
                .as_extension_chain
                .iter()
                .rfold(request.into(), |request, extension| {
                    NearPromise::new(extension)
                        .function_call(
                            FunctionCall::name("w_execute_extension")
                                .attach_deposit(NearToken::from_yoctonear(1))
                                .gas(request.estimate_gas())
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

    /// [Sign](Self::sign) the given on-chain [request](Request) to be
    /// executed on behalf of [effective account ID](Self::account_id) and
    /// relay it.
    ///
    /// # Panics
    ///
    /// This method panics if relayer is not [configured](Self::with_relayer)
    /// for this wallet.
    pub async fn sign_and_send(
        &self,
        request: impl Into<Request>,
    ) -> Result<defuse_near_sender::SentTransaction, Error> {
        // check before signing if relayer is set
        let relayer = self.relayer();

        let (msg, proof) = self.sign(request).await?;

        let mut req = WalletRelayRequest::new(msg, proof);

        if !self.initialized {
            req = req.deterministic_state_init(self.deterministic_state_init().clone());
        }

        relayer.relay_wallet_msg(req).await.map_err(Error::Relayer)
    }

    #[allow(clippy::doc_markdown)]
    /// Create a new MPC signer for curve with given domain that can sign
    /// arbitrary payloads on behalf of [effective account ID](Self::account_id).
    ///
    /// # Panics
    ///
    /// This method panics if [MPC contract ID](Self::with_mpc_contract_id),
    /// [relayer](Self::with_relayer) and [client](Self::with_client) are not
    /// configured for this wallet.
    ///
    /// # Examples
    ///
    /// On-chain MPC signer currently supports `ed25519` and `secp256k1` curves:
    ///
    /// ## EdDSA
    ///
    /// ```rust,no_run
    /// use defuse_wallet_sdk::mpc::kdf::{
    ///     DeriveSigner, crypto::{Curve, ed25519::Ed25519},
    /// };
    /// # use defuse_wallet_sdk::{Wallet, AccountIdRef};
    /// # use defuse_wallet_ed25519::{
    /// #   WalletEd25519, WalletEd25519Signer,
    /// #   crypto::ed25519::ed25519_dalek::SigningKey,
    /// # };
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let wallet = Wallet::<WalletEd25519>::new(
    /// #     [0u8; 32],
    /// #     WalletEd25519Signer(SigningKey::from_bytes(&[0u8; 32])),
    /// # );
    ///
    /// // prepare signer for Ed25519 curve
    /// let signer = wallet.mpc_signer::<Ed25519>(1).await?;
    ///
    /// // derive public key
    /// let path = "derivation path";
    /// let public_key = signer.derive_public_key(path);
    ///
    /// // sign arbitrary message
    /// let msg = b"some message";
    /// let signature = signer.derive_sign(path, msg).await?;
    ///
    /// assert!(Ed25519::verify(&public_key, msg, &signature));
    /// # Ok(()) }
    /// ```
    ///
    /// ## ECDSA
    ///
    /// ```rust,no_run
    /// use defuse_wallet_sdk::mpc::kdf::{
    ///     DeriveSigner, RecoverableDeriveSigner,
    ///     crypto::{RecoverableCurve, secp256k1::Secp256k1},
    /// };
    /// # use defuse_wallet_sdk::{Wallet, AccountIdRef};
    /// # use defuse_wallet_ed25519::{
    /// #   WalletEd25519, WalletEd25519Signer,
    /// #   crypto::ed25519::ed25519_dalek::SigningKey,
    /// # };
    /// # use hex_literal::hex;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let wallet = Wallet::<WalletEd25519>::new(
    /// #     [0u8; 32],
    /// #     WalletEd25519Signer(SigningKey::from_bytes(&[0u8; 32])),
    /// # );
    ///
    /// // prepare signer for secp256k1 curve
    /// let signer = wallet.mpc_signer::<Secp256k1>(0).await?;
    ///
    /// // derive public key
    /// let path = "derivation path";
    /// let public_key = signer.derive_public_key(path);
    ///
    /// // sign **32-byte prehash** (recoverable)
    /// let prehash = hex!("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    /// let (signature, recovery_id) = signer.derive_sign_recoverable(path, &prehash).await?;
    ///
    /// assert_eq!(
    ///     Secp256k1::recover(&prehash, &signature, recovery_id),
    ///     Some(public_key),
    /// );
    /// # Ok(()) }
    /// ```
    #[cfg(all(feature = "mpc", feature = "near-kit"))]
    pub async fn mpc_signer<C>(
        &self,
        domain_id: u64,
    ) -> Result<mpc::MpcOnChainSigner<C>, mpc::Error>
    where
        C: mpc::OnChainNearMpcCurve,
        S: 'static,
    {
        mpc::MpcOnChainSigner::from_domain_id(
            self.clone(),
            self.mpc_contract_id.clone().expect(
                "mpc_contract_id is not configured, use `with_mpc_contract_id()` to set one",
            ),
            domain_id,
            self.client(),
        )
        .await
    }

    /// Reseed the nonces and invalidate the current block. Use it in case of a collision.
    #[inline]
    pub fn reseed_nonces(&self) {
        *self.nonces.lock().unwrap() = ConcurrentNonces::new(make_rng());
    }
}

impl<S: SignatureSchema> AsRef<AccountIdRef> for Wallet<S> {
    /// Returns [effective account ID](Self::account_id) of the wallet.
    #[inline]
    fn as_ref(&self) -> &AccountIdRef {
        self.account_id()
    }
}

impl<S: SignatureSchema> From<&Wallet<S>> for AccountId {
    /// Coverts to [effective account ID](Wallet::account_id) of the wallet.
    #[inline]
    fn from(wallet: &Wallet<S>) -> Self {
        wallet.account_id().clone()
    }
}

impl<S: SignatureSchema> From<Wallet<S>> for AccountId {
    /// Coverts to [effective account ID](Wallet::account_id) of the wallet.
    #[inline]
    fn from(wallet: Wallet<S>) -> Self {
        (&wallet).into()
    }
}
/// An error returned from [`Wallet`] methods
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error occurred during [relaying](WalletRelayer::relay_wallet_msg)
    /// signed [request](RequestMessage).
    #[error("relayer: {0}")]
    Relayer(Box<dyn StdError + Send + Sync>),

    /// An error occurred during [signing](WalletSigner::sign_wallet_msg)
    /// wallet [request](RequestMessage).
    #[error("signer: {0}")]
    Signer(Box<dyn StdError + Send + Sync>),
}

impl<S> NearSender for Wallet<S>
where
    S: SignatureSchema,
{
    type Error = Error;

    #[inline]
    fn account_id(&self) -> Cow<'_, AccountIdRef> {
        self.account_id().into()
    }

    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error> {
        self.sign_and_send(NearPromise::new(receiver_id).add_actions(actions))
            .await
    }
}
