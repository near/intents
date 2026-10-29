use defuse_near_sender::SentTransaction;
use near_kit::{
    Error, SigningNear,
    protocol::{Action, derive_deterministic_account_id},
    transaction::{Included, WaitLevel},
};

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
    ) -> Result<SentTransaction, Self::Error> {
        if request.msg.chain_id != self.near().chain_id().as_str() {
            return Err(Error::Validation("invalid chain_id".to_string()));
        }

        if request.msg.request.estimate_gas() > MAX_GAS {
            return Err(Error::Validation("gas limit exceeded".to_string()));
        }

        let relayer = self
            .clone()
            // rely on timeouts instead of number of retry attempts
            .max_nonce_retries(u32::MAX)
            .wait_until::<Included>();
        let mut tx = relayer.transaction(&request.msg.signer_id);

        if let Some(state_init) = request.deterministic_state_init {
            if derive_deterministic_account_id(&state_init) != request.msg.signer_id {
                return Err(Error::Validation("invalid state_init".to_string()));
            }

            tx = tx.add_action(Action::state_init(
                state_init,
                // wallet-contract should fit into ZBA limits
                NearToken::ZERO,
            ));
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
