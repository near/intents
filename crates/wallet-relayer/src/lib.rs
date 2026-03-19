mod adapters;

use std::time::Duration;

use chrono::{TimeDelta, Utc};
use defuse_wallet::signature::RequestMessage;
use futures::TryFutureExt;
use near_kit::{Near, NearToken, Signer, TransactionOutcome, TxExecutionStatus};
use near_sdk::{Gas, near, serde_json::json, state_init::StateInit};
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct Relayer {
    client: Near,
    max_assist_deposit: NearToken,
    gas: Gas,
}

#[near(serializers = [json])] // TODO: get rid of `#[near]`?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInit>,
    pub msg: RequestMessage,
    pub proof: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl Relayer {
    /// Only assist with at most 1yN: it's enough for a single permissioned
    /// action on Near: most contracts require 1yN of attached deposit to
    /// ensure predecessor is not using FunctionCall access key
    const MAX_ASSIST_DEPOSIT_DEFAULT: NearToken = NearToken::from_yoctonear(1);

    const GAS_DEFAULT: Gas = Gas::from_tgas(300);

    pub fn new(rpc_url: impl Into<String>, signer: impl Signer + 'static) -> Self {
        // rely on timeouts instead of number of retry attempts
        const MAX_NONCE_RETRIES: u32 = u32::MAX;

        Self {
            client: Near::custom(rpc_url)
                .max_nonce_retries(MAX_NONCE_RETRIES)
                // TODO: derive multiple access keys?
                .signer(signer)
                .build(),
            max_assist_deposit: Self::MAX_ASSIST_DEPOSIT_DEFAULT,
            gas: Self::GAS_DEFAULT,
        }
    }

    pub const fn max_assist_deposit(mut self, deposit: NearToken) -> Self {
        self.max_assist_deposit = deposit;
        self
    }

    pub const fn gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    /// Relay signed request with optional attached deposit.
    /// If no additional deposit is needed then pass `NearToken::ZERO`.
    // TODO: return request hash?
    pub async fn relay(
        &self,
        request: RelayRequest,
        deposit: NearToken,
        max_gas: impl Into<Option<Gas>>,
        wait_until: TxExecutionStatus,
    ) -> Result<TransactionOutcome> {
        if request.msg.chain_id != self.client.network().as_str() {
            return Err(Error::InvalidChainId);
        }

        let mut tx = self.client.transaction(request.msg.signer_id.clone());

        let needs_deposit = if let Some(state_init) = request.state_init {
            if state_init.derive_account_id() != request.msg.signer_id {
                return Err(Error::InvalidStateInit);
            }

            tx = tx.state_init(adapters::state_init(state_init), NearToken::ZERO);

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

        tx = tx
            .call("w_execute_signed")
            .deposit(
                needs_deposit
                    // assist with deposit, but capped so the relayer will not get drained
                    .min(self.max_assist_deposit)
                    // attach optional given deposit, too
                    .saturating_add(deposit),
            )
            .gas(self.tx_gas(&request.msg, request.min_gas, max_gas)?)
            .args(json!({ // TODO: contract trait
                "msg": request.msg,
                "proof": request.proof,
            }))
            .finish();

        tokio::time::timeout(
            Self::tx_timeout(&request.msg)?,
            tx.wait_until(wait_until)
                .send()
                .into_future()
                .map_err(Error::Transaction),
        )
        .await
        .map_err(|_| Error::ExpiredOrFuture)
        // TODO: replace with `.flatten()` from rust 1.89
        .and_then(::core::convert::identity)
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

    fn tx_timeout(msg: &RequestMessage) -> Result<Duration> {
        /// Signers are recommended to set `created_at` a bit in the past,
        /// so that transaction doesn't fail on-chain due to possible lag
        /// in block timestamps.
        const SIGNER_LAG: TimeDelta = TimeDelta::seconds(60);

        let msg_timeout = TimeDelta::from_std(msg.timeout).map_err(|_| Error::InvalidTimeout)?;

        let deadline = msg
            .created_at
            .into_timestamp()
            .checked_add_signed(msg_timeout)
            .ok_or(Error::InvalidTimeout)?
            // add more buffer for short-living requests
            .checked_add_signed(SIGNER_LAG)
            .ok_or(Error::ExpiredOrFuture)?;

        deadline
            .signed_duration_since(Utc::now())
            .to_std()
            .map_err(|_| Error::ExpiredOrFuture)
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
    #[error("invalid timeout")]
    InvalidTimeout,
    #[error("invalid state_init")]
    InvalidStateInit,
    #[error("transaction: {0}")]
    Transaction(#[from] near_kit::Error),
}
