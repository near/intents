//! Wallet Relayer

use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use defuse_wallet::{RequestMessage, StateInit};
use impl_tools::autoimpl;
use near_kit::{
    Error, Gas, NearToken, SigningNear,
    protocol::Action,
    rpc::{BlockReference, Finality, RpcError, SendTxResponse},
    transaction::{Included, WaitLevel},
};
use serde::{Deserialize, Serialize};

use crate::{BLOCKCHAIN_LAG, Proof, client::WalletContract};

/// Request for [relaying](WalletRelayer) messages
/// to [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed)
/// method on [`msg.signer_id`](field@RequestMessage::signer_id) contract.
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletRelayRequest {
    /// Optionally initialize the contract before calling
    /// [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed).
    ///
    /// This is only required for the first transaction. If the contract is
    /// already initialized, this is no-op.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic_state_init: Option<StateInit>,

    /// Signed message
    pub msg: RequestMessage,

    /// Proof (i.e. signature)
    pub proof: Proof,
    // TODO: gas hint?
}

impl WalletRelayRequest {
    #[must_use]
    #[inline]
    pub fn new(msg: RequestMessage, proof: impl Into<String>) -> Self {
        Self {
            deterministic_state_init: None,
            msg,
            proof: proof.into(),
        }
    }

    #[must_use]
    #[inline]
    pub fn deterministic_state_init(mut self, state_init: impl Into<StateInit>) -> Self {
        self.deterministic_state_init = Some(state_init.into());
        self
    }
}

/// Relayer for [`Wallet`](crate::Wallet) requests
#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait WalletRelayer: Sync {
    type Error: Debug + Display;

    // TODO: ask for compensation?
    // async fn cover_gas_costs(&self, gas: Gas) -> Result<Vec<NearPromise>, Self::Error>;

    /// Relay wallet request to [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed)
    /// method on [`msg.signer_id`](field@crate::RequestMessage::signer_id) contract
    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SendTxResponse, Self::Error>;
}

impl<W: WaitLevel> WalletRelayer for SigningNear<W> {
    type Error = Error;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(
            relayer.account_id = %self.account_id(),
            msg.signer_id = %request.msg.signer_id,
            msg.hash = %near_kit::CryptoHash::from_bytes(request.msg.hash()),
        )))]
    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SendTxResponse, Self::Error> {
        const MAX_GAS: Gas = Gas::from_pgas(1);

        if request.msg.chain_id != self.near().chain_id().as_str() {
            return Err(Error::Validation("invalid chain_id".to_string()));
        }

        if request.msg.request.estimate_gas() > MAX_GAS {
            return Err(Error::Validation("gas limit exceeded".to_string()));
        }

        let mut tx = self
            .wait_until::<Included>()
            // rely on timeouts instead of number of retry attempts
            .max_nonce_retries(u32::MAX)
            .transaction(&request.msg.signer_id);

        let initialized = match self
            .near()
            .rpc()
            .view_account(tx.receiver_id(), BlockReference::Finality(Finality::Final))
            .await
        {
            Ok(account) => account.has_contract(),
            Err(RpcError::AccountNotFound { .. } | RpcError::UninitializedAccount { .. }) => false,
            Err(err) => return Err(err.into()),
        };

        if !initialized {
            let Some(state_init) = request.deterministic_state_init else {
                return Err(Error::Validation("uninitialized account".to_string()));
            };
            if state_init.derive_account_id() != request.msg.signer_id {
                return Err(Error::Validation("invalid state_init".to_string()));
            }

            tx = tx.add_action(Action::state_init(
                state_init,
                // wallet-contract should fit into ZBA limits
                NearToken::ZERO,
            ));
        }

        tx = tx.add_action(
            WalletContract::w_execute_signed(&request.msg, &request.proof).gas(MAX_GAS),
        );

        tokio::time::timeout(
            request
                .msg
                .duration_left()
                .ok_or_else(|| Error::Validation("expired or from the future".to_string()))?
                // add more buffer for short-living requests
                .saturating_add(BLOCKCHAIN_LAG),
            tx,
        )
        .await
        .map_err(|_| Error::Validation("expired".to_string()))
        .flatten()
        .map(Into::into)
    }
}

#[async_trait]
pub(crate) trait DynWalletRelayer: Send + Sync {
    async fn dyn_relay_signed_msg(
        &self,
        request: WalletRelayRequest,
    ) -> anyhow::Result<SendTxResponse>;
}

#[async_trait]
impl<R> DynWalletRelayer for R
where
    R: WalletRelayer,
    R::Error: StdError + Send + Sync + 'static,
{
    async fn dyn_relay_signed_msg(
        &self,
        request: WalletRelayRequest,
    ) -> anyhow::Result<SendTxResponse> {
        self.relay_wallet_msg(request).await.map_err(Into::into)
    }
}

impl WalletRelayer for dyn DynWalletRelayer + '_ {
    type Error = anyhow::Error;

    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SendTxResponse, Self::Error> {
        self.dyn_relay_signed_msg(request).await
    }
}
