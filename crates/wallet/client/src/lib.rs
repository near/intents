#[cfg(feature = "ed25519")]
pub mod ed25519;

use std::time::Duration;

pub use defuse_wallet as wallet;

use defuse_wallet::{
    ConcurrentNonces, Request, State,
    signature::{Deadline, RequestMessage},
};
use near_sdk::{
    AccountId, GlobalContractId,
    borsh::BorshSerialize,
    state_init::{StateInit, StateInitV1},
};
use rand::{make_rng, rngs::SmallRng};

pub const MAINNET: &str = "mainnet";

#[derive(Debug)]
pub struct WalletClient<S: Signer> {
    chain_id: String,
    global_contract_id: GlobalContractId,
    init_state: State<S::PublicKey>,
    nonces: ConcurrentNonces<SmallRng>,
    signer: S,
}

impl<S> WalletClient<S>
where
    S: Signer,
{
    pub fn new(global_contract_id: GlobalContractId, signer: S) -> Self {
        Self {
            chain_id: MAINNET.to_string(),
            global_contract_id,
            init_state: State::new(signer.public_key()),
            nonces: ConcurrentNonces::new(make_rng()),
            signer,
        }
    }

    #[must_use]
    pub fn with_chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    #[must_use]
    pub fn with_wallet_id(mut self, wallet_id: u32) -> Self {
        self.init_state = self.init_state.wallet_id(wallet_id);
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.init_state = self.init_state.timeout(timeout);
        self
    }

    #[must_use]
    pub fn with_extensions(mut self, account_ids: impl IntoIterator<Item = AccountId>) -> Self {
        self.init_state = self.init_state.extensions(account_ids);
        self
    }

    pub fn chain_id(&self) -> &str {
        self.chain_id.as_str()
    }

    pub const fn global_contract_id(&self) -> &GlobalContractId {
        &self.global_contract_id
    }

    pub const fn wallet_id(&self) -> u32 {
        self.init_state.wallet_id
    }

    pub const fn timeout(&self) -> Duration {
        self.init_state.nonces.timeout()
    }

    pub const fn signer(&self) -> &S {
        &self.signer
    }

    pub fn state_init(&self) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: self.global_contract_id.clone(),
            data: self.init_state.as_storage(),
        })
    }

    pub fn account_id(&self) -> AccountId {
        self.state_init().derive_account_id()
    }

    pub fn sign(&mut self, request: Request) -> (RequestMessage, Signature) {
        let msg = self.wrap_request_msg(request);
        let signature = self.signer.sign(&msg);
        (msg, signature)
    }

    fn wrap_request_msg(&mut self, request: Request) -> RequestMessage {
        RequestMessage {
            chain_id: self.chain_id.clone(),
            signer_id: self.account_id(),
            nonce: self.nonces.next().expect("failed to get next nonce"),
            created_at: Deadline::now() - Duration::from_secs(60),
            timeout: self.init_state.nonces.timeout(),
            request,
        }
    }
}

impl<S> Clone for WalletClient<S>
where
    S: Signer + Clone,
    S::PublicKey: Clone,
{
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id.clone(),
            global_contract_id: self.global_contract_id.clone(),
            init_state: self.init_state.clone(),
            nonces: ConcurrentNonces::new(make_rng()),
            signer: self.signer.clone(),
        }
    }
}

type Signature = String;

pub trait Signer {
    type PublicKey: BorshSerialize;

    fn public_key(&self) -> Self::PublicKey;
    // TODO: async? return Result<_>?
    fn sign(&self, msg: &RequestMessage) -> Signature;
}
