mod adapters;
mod contract;

use std::time::Duration;

pub use defuse_wallet as wallet;
pub use near_kit;

use chrono::{TimeDelta, Utc};
use near_kit::{
    CryptoHash, FinalExecutionOutcome, Gas, InvalidTxError, Near, NearToken, TxExecutionStatus,
};
use near_sdk::state_init::StateInit;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tracing::{field, instrument};

use crate::contract::{WExecuteSignedArgs, Wallet};

use self::wallet::signature::RequestMessage;

#[derive(Debug)]
pub struct Relayer {
    client: Near,
    gas: Gas,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInit>,
    pub msg: RequestMessage,
    pub proof: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl Relayer {
    #[allow(clippy::doc_markdown)]
    // TODO: remove once https://github.com/near/nearcore/pull/15461 is on mainnet
    /// Only assist with at most 1yN: it's enough for a single permissioned
    /// action on Near: most contracts require 1yN of attached deposit to
    /// ensure predecessor is not using FunctionCall access key
    const MAX_ASSIST_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

    // TODO: change to 1PGas once protocol version 83 is on mainnet
    // https://github.com/near/nearcore/releases/tag/2.11.0
    const GAS_DEFAULT: Gas = Gas::from_tgas(300);

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

    pub fn client(&self) -> Near {
        self.client.clone()
    }

    /// Relay signed request with optional attached deposit.
    /// If no additional deposit is needed then pass `NearToken::ZERO`.
    // TODO: return request hash?
    #[instrument(skip_all, fields(
        signer_id = %request.msg.signer_id,
        request.hash = %CryptoHash::from_bytes(request.msg.hash()),
        deposit = Some(deposit).filter(|d| !d.is_zero()).map(field::display),
    ))]
    pub async fn w_execute_signed(
        &self,
        request: RelayRequest,
        deposit: NearToken,
        max_gas: impl Into<Option<Gas>>,
    ) -> Result<FinalExecutionOutcome> {
        // TODO: replace with `self.client.chain_id().as_str()`
        if request.msg.chain_id != near_kit::ChainId::mainnet().as_str() {
            return Err(Error::InvalidChainId);
        }

        let mut tx = self.client.transaction(request.msg.signer_id.clone());

        let needs_deposit = if let Some(state_init) = request.state_init {
            if state_init.derive_account_id() != request.msg.signer_id {
                return Err(Error::InvalidStateInit);
            }

            tx = tx.state_init(
                adapters::state_init(state_init),
                NearToken::ZERO, // wallet-contract should fit into ZBA limits
            );

            // Here we're not sure if the contract was initialized already or not,
            // so we assist with the deposit if needed.
            // This is only usefull at the first initialization of the deterministic
            // account id, since 30% of used gas is funnelled back to the contract's
            // balance only at the end of receipt execution.
            // We could have checked whether the acccount exists already, but this would
            // increase the latency due to additional RPC call is needed.
            request.msg.request.out.total_deposit()
        } else {
            NearToken::ZERO
        };

        tx = tx.add_action(
            Wallet::w_execute_signed(WExecuteSignedArgs {
                msg: &request.msg,
                proof: &request.proof,
            })
            .deposit(
                needs_deposit
                    // assist with deposit, but capped so the relayer will not get drained
                    .min(Self::MAX_ASSIST_DEPOSIT)
                    // attach optional given deposit, too
                    .saturating_add(deposit),
            )
            .gas(self.tx_gas(&request.msg, request.min_gas, max_gas)?),
        );

        tokio::time::timeout(
            Self::request_timeout(&request.msg)?,
            tx.send()
                // wait for execution, so we have an access to wallet's receipt
                .wait_until(TxExecutionStatus::ExecutedOptimistic)
                // rely on timeouts instead of number of retry attempts
                .max_nonce_retries(u32::MAX),
        )
        .await
        .map_err(|_| Error::Expired)?
        .map_err(Error::Transaction)
    }

    fn tx_gas(
        &self,
        msg: &RequestMessage,
        request_gas: Option<Gas>,
        max_gas: impl Into<Option<Gas>>,
    ) -> Result<Gas> {
        let max_gas = max_gas.into().unwrap_or(self.gas);

        if request_gas.unwrap_or_else(|| msg.request.out.estimate_gas()) > max_gas {
            return Err(Error::GasLimit);
        }

        Ok(max_gas)
    }

    fn request_timeout(msg: &RequestMessage) -> Result<Duration> {
        /// Signers are recommended to set `created_at` a bit in the past,
        /// so that transaction doesn't fail on-chain due to possible lag
        /// in block timestamps.
        const SIGNER_LAG: TimeDelta = TimeDelta::seconds(60);

        let timeout = TimeDelta::from_std(msg.timeout).map_err(|_| Error::InvalidTimeout)?;

        if !msg.created_at.has_expired() {
            return Err(Error::FromTheFuture);
        }

        let deadline = msg
            .created_at
            .into_timestamp()
            .checked_add_signed(timeout)
            .ok_or(Error::InvalidTimeout)?
            // add more buffer for short-living requests
            .checked_add_signed(SIGNER_LAG)
            .ok_or(Error::InvalidTimeout)?;

        deadline
            .signed_duration_since(Utc::now())
            .to_std()
            .map_err(|_| Error::Expired)
    }
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("expired")]
    Expired,
    #[error("from the future")]
    FromTheFuture,
    #[error("gas limit exceeded")]
    GasLimit,
    #[error("invalid chain_id")]
    InvalidChainId,
    #[error("invalid timeout")]
    InvalidTimeout,
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
