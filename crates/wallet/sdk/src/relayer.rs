use std::{borrow::Cow, sync::Arc};

use defuse_wallet::{
    AccountId, AccountIdRef, NearPromise, SignatureSchema,
    actions::{FunctionCall, NearAction},
};
use impl_tools::autoimpl;

use crate::{Wallet, WalletSigner, client::WExecuteSignedArgs};

#[autoimpl(Deref using self.wallet)]
pub struct RelayedWallet<SS, S, R> {
    wallet: Wallet<SS, S>,
    relayer: R,
}

impl<SS, S, R> Relayer for RelayedWallet<SS, S, R>
where
    SS: SignatureSchema,
    S: WalletSigner<SS>,
    R: Relayer,
{
    type Error = R::Error; // TODO: or signer error?

    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error> {
        let (msg, proof) = self
            .wallet
            .sign(
                actions
                    .into_iter()
                    .fold(NearPromise::new(receiver_id), |promise, action| {
                        promise.add_action(action)
                    }),
            )
            .await
            // TODO: return signer error
            .unwrap();

        self.relayer
            .send(
                msg.signer_id.clone(),
                vec![
                    FunctionCall::name("w_execute_signed")
                        .args_json(WExecuteSignedArgs::from((msg, proof)))
                        // TODO: gas + deposit?
                        .into(),
                ],
            )
            .await
    }
}

impl<SS, S, R> FixedRelayer for RelayedWallet<SS, S, R>
where
    SS: SignatureSchema,
    S: WalletSigner<SS>,
    R: Relayer,
{
    #[inline]
    fn account_id(&self) -> Cow<'_, AccountIdRef> {
        self.wallet.account_id().into()
    }
}

#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait Relayer: Sync {
    type Error;

    // TODO: ask for compensation?

    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error>;
}

pub trait FixedRelayer: Relayer {
    fn account_id(&self) -> Cow<'_, AccountIdRef>;
}

/// TODO: docs
#[derive(Debug, Clone, PartialEq, Eq)]
// TODO: serialize
pub struct SentTransaction {
    pub tx_hash: [u8; 32],
    // TODO: docs: must be the same as Sender::account_id
    pub sender_id: AccountId,
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::{Error, Included, Near, SendTxResponse};

    impl Relayer for Near {
        type Error = Error;

        async fn send(
            &self,
            receiver_id: AccountId,
            actions: Vec<NearAction>,
        ) -> Result<SentTransaction, Self::Error> {
            // TODO: reuse logic from relayer

            actions
                .into_iter()
                .fold(self.transaction(receiver_id), |tx, action| {
                    tx.add_action(action)
                })
                .send()
                // TODO: maybe IncludedFinal?
                .wait_until(Included)
                // TODO: max_nonce_retries
                .await
                .map(Into::into)
        }
    }

    impl FixedRelayer for Near {
        #[inline]
        fn account_id(&self) -> Cow<'_, AccountIdRef> {
            self.account_id().into()
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
