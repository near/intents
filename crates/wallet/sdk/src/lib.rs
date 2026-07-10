mod nonces;
mod signer;

pub use self::{nonces::*, signer::*};

use std::{
    collections::BTreeSet,
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};

use borsh::BorshSerialize;
pub use defuse_wallet_core::*;

use near_global_contracts::{GlobalContractId, StateInit, StateInitV1};
use rand::{make_rng, rngs::SmallRng};

pub const MAINNET: &str = "mainnet";

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
    #[inline]
    pub const fn new() -> Self {
        Self {
            subwallet_id: DEFAULT_SUBWALLET_ID,
            timeout: DEFAULT_TIMEOUT,
            extensions: BTreeSet::new(),
        }
    }

    #[inline]
    pub const fn subwallet_id(mut self, subwallet_id: u32) -> Self {
        self.subwallet_id = subwallet_id;
        self
    }

    #[inline]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[inline]
    pub fn extensions(mut self, account_ids: impl IntoIterator<Item = AccountId>) -> Self {
        self.extensions.extend(account_ids);
        self
    }

    pub fn build<SS, S>(self, code: impl Into<GlobalContractId>, signer: S) -> WalletSigner<SS, S>
    where
        SS: SignatureSchema<PublicKey: BorshSerialize>,
        S: Signer<SS>,
    {
        let state_init = StateInit::V1(StateInitV1 {
            code: code.into(),
            data: State::new(signer.public_key())
                .subwallet_id(self.subwallet_id)
                .timeout(self.timeout)
                .extensions(self.extensions)
                .as_storage(),
        });

        WalletSigner {
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

// TODO: avoid requiring SS to implement derived traits
#[derive(Debug, Clone)]
pub struct WalletSigner<SS: SignatureSchema, S: Signer<SS>> {
    account_id: AccountId,
    state_init: StateInit,

    subwallet_id: u32,
    timeout: Duration,
    nonces: Arc<Mutex<ConcurrentNonces<SmallRng>>>,

    signer: S,
    _schema: PhantomData<SS>,
}

impl<SS, S> WalletSigner<SS, S>
where
    SS: SignatureSchema,
    S: Signer<SS>,
{
    #[inline]
    pub const fn builder() -> WalletBuilder {
        WalletBuilder::new()
    }

    #[inline]
    pub fn new(code: impl Into<GlobalContractId>, signer: S) -> Self
    where
        SS::PublicKey: BorshSerialize,
    {
        Self::builder().build(code, signer)
    }

    #[inline]
    pub const fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    #[inline]
    pub const fn deterministic_state_init(&self) -> &StateInit {
        &self.state_init
    }

    #[inline]
    pub const fn subwallet_id(&self) -> u32 {
        self.subwallet_id
    }

    #[inline]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[inline]
    pub const fn signer(&self) -> &S {
        &self.signer
    }

    #[inline]
    pub fn public_key(&self) -> SS::PublicKey {
        self.signer().public_key()
    }

    #[allow(clippy::future_not_send)]
    pub async fn sign(
        &self,
        request: Request,
        chain_id: impl Into<String>,
    ) -> Result<(RequestMessage, Proof), S::Error> {
        let msg = self.wrap_request_msg(request, chain_id);
        let proof = self.signer.sign(&msg).await?;

        debug_assert!(
            SS::verify(&self.signer.public_key(), &msg, &proof),
            "signer produced invalid signature",
        );

        Ok((msg, proof))
    }

    /// Wraps [`Request`] in [`RequestMessage`] for signing
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

    /// Reseed the nonces and invalidate the current block.
    /// Use it in case of a collision.
    #[inline]
    pub fn reseed_nonces(&self) {
        *self.nonces.lock().unwrap() = ConcurrentNonces::new(make_rng());
    }
}
