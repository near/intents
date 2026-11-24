use near_kit::{
    Error, SigningNear,
    protocol::Action,
    rpc::{BlockReference, Finality, RpcError, SendTxResponse},
    transaction::{Included, WaitLevel},
};

use crate::{
    BLOCKCHAIN_LAG, Gas, NearToken,
    client::WalletContract,
    relayer::{WalletRelayRequest, WalletRelayer},
};

// TODO: change to 1 PGas
const MAX_GAS: Gas = Gas::from_tgas(300);

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
