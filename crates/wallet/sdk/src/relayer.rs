use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_wallet::{
    AccountId, AccountIdRef, NearPromise, Request, RequestMessage, SignatureSchema,
    actions::NearAction,
};
use impl_tools::autoimpl;

use crate::{Proof, Wallet, WalletSigner};

#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait WalletRelayer: Sync {
    type Error: Debug + Display;

    // TODO: chain_id?

    // TODO: ask for compensation?

    // TODO: state_init?
    async fn relay_signed_msg(
        &self,
        msg: RequestMessage,
        proof: Proof,
    ) -> Result<SentTransaction, Self::Error>;
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::{Error, Included, Near, SendTxResponse};

    impl WalletRelayer for Near {
        type Error = Error;

        async fn relay_signed_msg(
            &self,
            msg: RequestMessage,
            proof: Proof,
        ) -> Result<SentTransaction, Self::Error> {
            use crate::client::WalletContract;

            self.transaction(msg.signer_id.clone())
                // TODO: state_init
                .add_action(
                    // TODO: gas + deposit?
                    WalletContract::w_execute_signed((msg, proof).into()),
                )
                .send()
                // TODO: maybe IncludedFinal?
                .wait_until(Included)
                // TODO: max_nonce_retries
                .await
                .map(Into::into)
        }
    }

    impl From<SendTxResponse> for SentTransaction {
        #[inline]
        fn from(value: SendTxResponse) -> Self {
            Self {
                tx_hash: *value.transaction_hash.as_bytes(),
                sender_id: value.sender_id,
            }
        }
    }
};

#[autoimpl(Deref using self.wallet)]
pub struct RelayedWallet<SS, S, R> {
    wallet: Wallet<SS, S>,
    relayer: R,
}

impl<SS, S, R> RelayedWallet<SS, S, R>
where
    SS: SignatureSchema,
    S: WalletSigner<SS>,
    R: WalletRelayer,
{
    // TODO: tracing
    pub async fn sign_and_relay(
        &self,
        request: impl Into<Request>,
    ) -> Result<SentTransaction, RelayedWalletError<S::Error, R::Error>> {
        let (msg, proof) = self
            .wallet
            .sign(request)
            .await
            .map_err(RelayedWalletError::Signer)?;

        self.relayer
            .relay_signed_msg(msg, proof)
            .await
            .map_err(RelayedWalletError::Relayer)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RelayedWalletError<S, R> {
    #[error("signer: {0}")]
    Signer(S),
    #[error("relayer: {0}")]
    Relayer(R),
}

#[cfg(feature = "mpc")]
const _: () = {
    use defuse_mpc_signer::Sender;

    impl<SS, S, R> Sender for RelayedWallet<SS, S, R>
    where
        SS: SignatureSchema,
        S: WalletSigner<SS>,
        R: WalletRelayer,
    {
        type Error = RelayedWalletError<S::Error, R::Error>;

        fn account_id(&self) -> Cow<'_, AccountIdRef> {
            self.wallet.account_id().into()
        }

        async fn send(
            &self,
            receiver_id: AccountId,
            actions: Vec<NearAction>,
        ) -> Result<defuse_mpc_signer::SentTransaction, Self::Error> {
            self.sign_and_relay(NearPromise::new(receiver_id).add_actions(actions))
                .await
                .map(Into::into)
        }
    }

    // TODO
    impl From<SentTransaction> for defuse_mpc_signer::SentTransaction {
        #[inline]
        fn from(value: SentTransaction) -> Self {
            Self {
                tx_hash: value.tx_hash,
                sender_id: value.sender_id,
            }
        }
    }
};

/// TODO: docs
#[derive(Debug, Clone, PartialEq, Eq)]
// TODO: serialize
pub struct SentTransaction {
    pub tx_hash: [u8; 32],
    // TODO: docs: must be the same as Sender::account_id
    pub sender_id: AccountId,
}
