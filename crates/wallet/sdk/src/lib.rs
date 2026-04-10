#[cfg(feature = "ed25519")]
pub mod ed25519;

use std::time::Duration;

pub use defuse_wallet as wallet;

use defuse_wallet::{
    ConcurrentNonces, Request, State,
    signature::{Deadline, RequestMessage},
};
use impl_tools::autoimpl;
use near_sdk::{AccountId, GlobalContractId, borsh::BorshSerialize, state_init::StateInit};
use rand::{make_rng, rngs::SmallRng};

pub const MAINNET: &str = "mainnet";

pub struct WalletSignerBuilder<S: Signer> {
    code: GlobalContractId,
    state: State<S::PublicKey>,
    signer: S,
}

impl<S: Signer> WalletSignerBuilder<S> {
    #[inline]
    pub fn new(code: GlobalContractId, signer: S) -> Self {
        Self {
            code,
            state: State::new(signer.public_key()),
            signer,
        }
    }

    #[must_use]
    #[inline]
    pub fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.state = self.state.wallet_id(wallet_id);
        self
    }

    #[must_use]
    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.state = self.state.timeout(timeout);
        self
    }

    #[must_use]
    #[inline]
    pub fn extensions(
        mut self,
        account_ids: impl IntoIterator<Item = impl Into<AccountId>>,
    ) -> Self {
        self.state = self.state.extensions(account_ids);
        self
    }

    #[inline]
    fn state_init(&self) -> StateInit {
        self.state.state_init(self.code.clone())
    }

    pub fn build(self) -> WalletSigner<S> {
        WalletSigner {
            chain_id: MAINNET.to_string(),
            account_id: self.state_init().derive_account_id(),
            code: self.code,
            state: self.state,
            nonces: ConcurrentNonces::new(make_rng()),
            signer: self.signer,
        }
    }
}

#[derive(Debug)]
#[autoimpl(Deref using self.state)]
pub struct WalletSigner<S: Signer> {
    chain_id: String,

    code: GlobalContractId,
    state: State<S::PublicKey>,

    account_id: AccountId,

    nonces: ConcurrentNonces<SmallRng>,
    signer: S,
}

impl<S> WalletSigner<S>
where
    S: Signer,
{
    #[inline]
    pub fn builder(code: GlobalContractId, signer: S) -> WalletSignerBuilder<S> {
        WalletSignerBuilder::new(code, signer)
    }

    #[inline]
    pub fn new(code: GlobalContractId, signer: S) -> Self {
        Self::builder(code, signer).build()
    }

    #[must_use]
    pub fn with_chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    pub fn chain_id(&self) -> &str {
        self.chain_id.as_str()
    }

    pub fn state_init(&self) -> StateInit {
        self.state.state_init(self.code.clone())
    }

    pub const fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub const fn signer(&self) -> &S {
        &self.signer
    }

    pub fn sign(&mut self, request: Request) -> Result<(RequestMessage, Proof), S::Error> {
        let msg = self.wrap_request_msg(request);
        let signature = self.signer.sign(&msg)?;
        Ok((msg, signature))
    }

    /// Wraps [`Request`] in [`RequestMessage`] for signing
    fn wrap_request_msg(&mut self, request: Request) -> RequestMessage {
        RequestMessage {
            chain_id: self.chain_id.clone(),
            signer_id: self.account_id().clone(),
            nonce: self.nonces.next(),
            // set `created_at` slightly before the actual time of signing,
            // so it doesn't fail on-chain if arrives too fast.
            created_at: Deadline::now() - self.optimal_lag(),
            timeout: self.state.nonces.timeout(),
            request,
        }
    }

    /// Returns an optimal lag for `created_at`, so it doesn't fail on-chain.
    fn optimal_lag(&self) -> Duration {
        Duration::from_secs(60).min(self.state.nonces.timeout() / 5)
    }
}

impl<S> Clone for WalletSigner<S>
where
    S: Signer + Clone,
    S::PublicKey: Clone,
{
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id.clone(),
            code: self.code.clone(),
            state: self.state.clone(),
            account_id: self.account_id.clone(),
            nonces: ConcurrentNonces::new(make_rng()),
            signer: self.signer.clone(),
        }
    }
}

/// Generic siagnature
pub type Proof = String;

pub trait Signer {
    type PublicKey: BorshSerialize;
    type Error;

    fn public_key(&self) -> Self::PublicKey;
    // TODO: async
    fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}
