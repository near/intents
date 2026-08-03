use defuse_near_sender::SentTransaction;
use near_kit::{Error, Included, Near};

use crate::{
    BLOCKCHAIN_LAG, Gas, NearToken,
    client::WalletContract,
    relayer::{WalletRelayRequest, WalletRelayer},
};

// TODO: remove once https://github.com/near/nearcore/pull/15461 is on mainnet
/// Only assist with at most 1yN: it's enough for a single permissioned
/// action on Near: most contracts require 1yN of attached deposit to
/// ensure predecessor is not using Function Call access key
const MAX_ASSIST_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

// TODO: change to 1 PGas
const MAX_GAS: Gas = Gas::from_tgas(300);

impl WalletRelayer for Near {
    type Error = Error;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(
            relayer.account_id = %self.account_id(),
            msg.signer_id = %request.msg.signer_id,
            msg.hash = %near_kit::CryptoHash::from_bytes(request.msg.hash()),
        )))]
    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Self::Error> {
        if request.msg.chain_id != self.chain_id().as_str() {
            return Err(Error::InvalidTransaction(
                TxError::InvalidChainId.to_string(),
            ));
        }

        if request.msg.request.estimate_gas() > MAX_GAS {
            return Err(Error::InvalidTransaction(TxError::GasLimit.to_string()));
        }

        let mut tx = self.transaction(&request.msg.signer_id);

        if let Some(state_init) = request.deterministic_state_init {
            if state_init.derive_account_id() != request.msg.signer_id {
                return Err(Error::InvalidTransaction(
                    TxError::InvalidStateInit.to_string(),
                ));
            }

            tx = tx.state_init(
                state_init,
                // wallet-contract should fit into ZBA limits
                NearToken::ZERO,
            );
        }

        tx = tx.add_action(
            WalletContract::w_execute_signed((&request.msg, request.proof).into())
                .deposit(request.msg.request.total_deposit().min(MAX_ASSIST_DEPOSIT))
                .gas(MAX_GAS),
        );

        tokio::time::timeout(
            request
                .msg
                .duration_left()
                .ok_or(TxError::ExpiredOrFuture)
                .map_err(|e| Error::InvalidTransaction(e.to_string()))?
                // add more buffer for short-living requests
                .saturating_add(BLOCKCHAIN_LAG),
            tx.send()
                .wait_until::<Included>()
                // rely on timeouts instead of number of retry attempts
                .max_nonce_retries(u32::MAX),
        )
        .await
        .map_err(|_| Error::InvalidTransaction(TxError::ExpiredOrFuture.to_string()))
        .flatten()
        .map(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
enum TxError {
    #[error("invalid chain_id")]
    InvalidChainId,
    #[error("invalid state_init")]
    InvalidStateInit,
    #[error("expired or from the future")]
    ExpiredOrFuture,
    #[error("gas limit exceeded")]
    GasLimit,
}
