mod request;

pub use self::request::*;

use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use defuse_near_sender::{NearSender, SentTransaction};
use defuse_wallet::{
    AccountId, AccountIdRef, NearPromise, NearToken, SignatureSchema, actions::NearAction,
};
use impl_tools::autoimpl;

use crate::{Error, Wallet};

#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait WalletRelayer: Sync {
    type Error: Debug + Display;

    // TODO: ask for compensation?
    // async fn cover_gas_costs(&self, gas: Gas) -> Result<Vec<NearPromise>, Self::Error>;

    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Self::Error>;

    #[inline]
    fn boxed<'a>(self) -> BoxWalletRelayer<'a>
    where
        Self: Sized + 'a,
        Self::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        Box::new(self)
    }

    #[inline]
    fn arced(self) -> ArcWalletRelayer
    where
        Self: Sized + 'static,
        Self::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        Arc::new(self)
    }
}

pub type BoxWalletRelayer<'a> = Box<dyn DynWalletRelayer + 'a>;
pub type ArcWalletRelayer = Arc<dyn DynWalletRelayer>;

impl<S> NearSender for Wallet<S>
where
    S: SignatureSchema,
{
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
        self.sign_and_send(NearPromise::new(receiver_id).add_actions(actions))
            .await
    }
}

#[async_trait]
pub trait DynWalletRelayer: Send + Sync {
    async fn dyn_relay_signed_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Box<dyn StdError + Send + Sync>>;
}

#[async_trait]
impl<R> DynWalletRelayer for R
where
    R: WalletRelayer<Error: Into<Box<dyn StdError + Send + Sync>>>,
{
    async fn dyn_relay_signed_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Box<dyn StdError + Send + Sync>> {
        self.relay_wallet_msg(request).await.map_err(Into::into)
    }
}

impl WalletRelayer for dyn DynWalletRelayer + '_ {
    type Error = Box<dyn StdError + Send + Sync>;

    async fn relay_wallet_msg(
        &self,
        request: WalletRelayRequest,
    ) -> Result<SentTransaction, Self::Error> {
        self.dyn_relay_signed_msg(request).await
    }
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::{Error, Gas, Included, Near};

    use crate::client::WalletContract;

    // TODO: remove once https://github.com/near/nearcore/pull/15461 is on mainnet
    /// Only assist with at most 1yN: it's enough for a single permissioned
    /// action on Near: most contracts require 1yN of attached deposit to
    /// ensure predecessor is not using Function Call access key
    const MAX_ASSIST_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

    const MAX_GAS: Gas = Gas::from_tgas(300);

    impl WalletRelayer for Near {
        type Error = Error;

        async fn relay_wallet_msg(
            &self,
            request: WalletRelayRequest,
        ) -> Result<SentTransaction, Self::Error> {
            // TODO: replace with `self.client.chain_id().as_str()`
            if request.msg.chain_id != near_kit::ChainId::mainnet().as_str() {
                return Err(Error::InvalidTransaction("invalid chain_id".to_string()));
            }

            let mut tx = self.transaction(&request.msg.signer_id);

            if let Some(state_init) = request.deterministic_state_init {
                if state_init.derive_account_id() != request.msg.signer_id {
                    return Err(Error::InvalidTransaction("invalid state_init".to_string()));
                }

                tx = tx.state_init(
                    state_init,
                    // wallet-contract should fit into ZBA limits
                    NearToken::ZERO,
                );
            }

            tx.add_action(
                WalletContract::w_execute_signed((&request.msg, request.proof).into())
                    .deposit(request.msg.request.total_deposit().min(MAX_ASSIST_DEPOSIT))
                    .gas(MAX_GAS),
            )
            .send()
            .wait_until(Included)
            .await
            .map(Into::into)
        }
    }
};
