pub use defuse_near_promise::*;

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_near_promise::actions::NearAction;
use impl_tools::autoimpl;

// TODO: docs
#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait NearSender: Sync {
    type Error: Debug + Display;

    // TODO: docs
    fn account_id(&self) -> Cow<'_, AccountIdRef>;

    // TODO: docs
    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error>;
}

/// TODO: docs
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentTransaction {
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))]
    pub tx_hash: [u8; 32],
    // TODO: docs: must be the same as Sender::account_id
    pub sender_id: AccountId,
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::{Action, Error, Included, Near, SendTxResponse};

    impl NearSender for Near {
        type Error = Error;

        #[inline]
        fn account_id(&self) -> Cow<'_, AccountIdRef> {
            self.account_id().into()
        }

        async fn send(
            &self,
            receiver_id: AccountId,
            actions: Vec<NearAction>,
        ) -> Result<SentTransaction, Self::Error> {
            use near_kit::TransactionBuilder;

            actions
                .into_iter()
                .map(Action::from)
                .fold(
                    self.transaction(receiver_id),
                    TransactionBuilder::add_action,
                )
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

    impl From<SentTransaction> for SendTxResponse {
        #[inline]
        fn from(value: SentTransaction) -> Self {
            Self {
                transaction_hash: value.tx_hash.into(),
                sender_id: value.sender_id,
            }
        }
    }
};
