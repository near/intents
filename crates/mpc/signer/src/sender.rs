use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_near_promise::{AccountId, AccountIdRef, actions::NearAction};
use impl_tools::autoimpl;
use near_kit::SendTxResponse;

#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait Sender: Sync {
    type Error: Debug + Display;

    // TODO: docs
    fn account_id(&self) -> Cow<'_, AccountIdRef>;

    // TODO: docs
    // TODO: actions
    async fn send(
        &self,
        receiver_id: AccountId,
        actions: Vec<NearAction>,
    ) -> Result<SentTransaction, Self::Error>;
}

/// TODO: docs
#[derive(Debug, Clone, PartialEq, Eq)]
// TODO: serialize
pub struct SentTransaction {
    pub tx_hash: [u8; 32],
    // TODO: docs: must be the same as Sender::account_id
    pub sender_id: AccountId,
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

// TODO: cfg(feature = "near-kit")
// impl Sender for Near
