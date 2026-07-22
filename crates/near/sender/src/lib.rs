use async_trait::async_trait;
pub use defuse_near_promise::*;

use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_near_promise::actions::NearAction;
use impl_tools::autoimpl;

/// A [_fixed_](Self::account_id) sender of arbitrary Near transactions or
/// promises
#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait NearSender: Sync {
    /// An error returned from [`.send()`](Self::send)
    type Error: Debug + Display;

    /// A **fixed** predecessor for target receipts.
    ///
    /// This method MUST return the same account ID for every call.
    ///
    /// See [`.send()`](Self::send) for details.
    fn account_id(&self) -> Cow<'_, AccountIdRef>;

    /// Send a transaction which SHOULD result in executing a receipt with
    /// given actions on the given receiver from a [**fixed**](Self::account_id)
    /// predecessor.
    ///
    /// This "target" receipt MIGHT NOT be the first or the last one - it can
    /// happen anywhere within the transaction. Moreover, there is no guarantee
    /// that the receipt will be created and successfully executed at all, since
    /// the transaction MAY fail at earlier stage.
    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error>;

    #[inline]
    fn boxed<'a>(self) -> BoxNearSender<'a>
    where
        Self: Sized + 'a,
        Self::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        Box::new(self)
    }

    #[inline]
    fn arced(self) -> ArcNearSender
    where
        Self: Sized + 'static,
        Self::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        Arc::new(self)
    }
}

/// A sent transaction identified by its [hash](field@Self::tx_hash) and
/// [sender](field@Self::sender_id).
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentTransaction {
    /// Hash of the sent transaction
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))]
    pub tx_hash: [u8; 32],

    /// A sender of the whole transaction, i.e. account ID that triggered the
    /// first receipt.
    pub sender_id: AccountId,
}

impl SentTransaction {
    #[cfg(feature = "near-kit")]
    pub async fn status<W: ::near_kit::WaitLevel>(
        self,
        client: &::near_kit::Near,
        level: W,
    ) -> Result<W::Response, ::near_kit::Error> {
        client
            .tx_status::<W>(&self.tx_hash.into(), self.sender_id, level)
            .await
    }
}

pub type BoxNearSender<'a> = Box<dyn DynNearSender + 'a>;
pub type ArcNearSender = Arc<dyn DynNearSender>;

/// A `dyn`-compatible adaptor for [`NearSender`]
#[async_trait]
pub trait DynNearSender: Send + Sync {
    fn dyn_account_id(&self) -> Cow<'_, AccountIdRef>;
    async fn dyn_send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Box<dyn StdError + Send + Sync>>;
}

#[async_trait]
impl<S> DynNearSender for S
where
    S: NearSender,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    #[inline]
    fn dyn_account_id(&self) -> Cow<'_, AccountIdRef> {
        self.account_id()
    }

    async fn dyn_send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Box<dyn StdError + Send + Sync>> {
        self.send(receiver_id, actions).await.map_err(Into::into)
    }
}

impl NearSender for dyn DynNearSender + '_ {
    type Error = Box<dyn StdError + Send + Sync>;

    #[inline]
    fn account_id(&self) -> Cow<'_, AccountIdRef> {
        self.dyn_account_id()
    }

    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error> {
        self.dyn_send(receiver_id, actions).await
    }
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
                .wait_until(Included)
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
