#[cfg(feature = "near-kit")]
mod near_kit;
mod request;

pub use self::request::*;

use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use defuse_near_sender::SentTransaction;
use impl_tools::autoimpl;

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
    ) -> Result<SentTransaction, Self::Error>;
}

#[async_trait]
pub(crate) trait DynWalletRelayer: Send + Sync {
    async fn dyn_relay_signed_msg(
        &self,
        request: WalletRelayRequest,
    ) -> anyhow::Result<SentTransaction>;
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
    ) -> anyhow::Result<SentTransaction> {
        self.relay_wallet_msg(request).await.map_err(Into::into)
    }
}

impl WalletRelayer for dyn DynWalletRelayer + '_ {
    type Error = anyhow::Error;

    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Self::Error> {
        self.dyn_relay_signed_msg(request).await
    }
}
