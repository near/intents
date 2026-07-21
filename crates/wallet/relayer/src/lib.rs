use std::{convert::Infallible, time::Duration};

pub use defuse_wallet_sdk as wallet;
use defuse_wallet_sdk::{client::WalletContract, relayer::WalletRelayRequest};
pub use near_kit;
use near_kit::{ExecutedOptimistic, FinalExecutionOutcome, Gas, InvalidTxError, Near, NearToken};
use thiserror::Error as ThisError;
#[cfg(feature = "tracing")]
use tracing::instrument;

#[derive(Debug)]
pub struct WalletRelayer {
    client: Near,
    gas: Gas,
}

/// Signers are recommended to set `created_at` a bit in the past,
/// so that transaction doesn't fail on-chain due to possible lag
/// in block timestamps.
const BLOCKCHAIN_LAG: Duration = Duration::from_mins(1);

impl WalletRelayer {
    #[allow(clippy::doc_markdown)]
    // TODO: remove once https://github.com/near/nearcore/pull/15461 is on mainnet
    /// Only assist with at most 1yN: it's enough for a single permissioned
    /// action on Near: most contracts require 1yN of attached deposit to
    /// ensure predecessor is not using FunctionCall access key
    const MAX_ASSIST_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

    const GAS_DEFAULT: Gas = Gas::from_pgas(1);

    pub const fn new(client: Near) -> Self {
        Self {
            client,
            gas: Self::GAS_DEFAULT,
        }
    }

    #[must_use]
    pub const fn gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    pub const fn client(&self) -> &Near {
        &self.client
    }

    /// Relay signed request with optional attached deposit.
    /// If no additional deposit is needed then pass `NearToken::ZERO`.
    #[cfg_attr(
        feature = "tracing",
        instrument(skip_all, fields(
            msg.signer_id = %request.msg.signer_id,
            msg.hash = %near_kit::CryptoHash::from_bytes(request.msg.hash()),
        ))
    )]
    pub async fn w_execute_signed(
        &self,
        request: WalletRelayRequest,
    ) -> Result<FinalExecutionOutcome> {
        // TODO: replace with `self.client.chain_id().as_str()`
        if request.msg.chain_id != near_kit::ChainId::mainnet().as_str() {
            return Err(Error::InvalidChainId);
        }

        let mut tx = self.client.transaction(request.msg.signer_id.clone());

        if let Some(state_init) = request.deterministic_state_init {
            if state_init.derive_account_id() != request.msg.signer_id {
                return Err(Error::InvalidStateInit);
            }

            tx = tx.state_init(
                state_init,
                // wallet-contract should fit into ZBA limits
                NearToken::ZERO,
            );
        }

        tx = tx.add_action(
            WalletContract::w_execute_signed((&request.msg, &request.proof).into())
                .deposit(
                    request
                        .msg
                        .request
                        .total_deposit()
                        // assist with deposit, but capped so the relayer will not get drained
                        .min(Self::MAX_ASSIST_DEPOSIT),
                )
                .gas(self.gas),
        );

        tokio::time::timeout(
            request
                .msg
                .time_left()
                .ok_or(Error::ExpiredOrFuture)?
                // add more buffer for short-living requests
                .saturating_add(BLOCKCHAIN_LAG),
            tx.send()
                // wait for execution, so we have an access to wallet's receipt
                .wait_until(ExecutedOptimistic)
                // rely on timeouts instead of number of retry attempts
                .max_nonce_retries(u32::MAX),
        )
        .await
        .map_err(|_| Error::ExpiredOrFuture)?
        .map_err(Error::Transaction)
    }
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("expired or from the future")]
    ExpiredOrFuture,
    #[error("gas limit exceeded")]
    GasLimit,
    #[error("invalid chain_id")]
    InvalidChainId,
    #[error("invalid state_init")]
    InvalidStateInit,
    #[error("transaction: {0}")]
    Transaction(#[from] near_kit::Error),
}

impl From<InvalidTxError> for Error {
    #[inline]
    fn from(err: InvalidTxError) -> Self {
        Self::Transaction(near_kit::Error::InvalidTx(err.into()))
    }
}

impl From<Infallible> for Error {
    #[inline]
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
