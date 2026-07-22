use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use defuse_wallet::{RequestMessage, SignatureSchema};
use impl_tools::autoimpl;

/// A proof for [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed)
pub type Proof = String;

/// A signer that can sign [`RequestMessage`] according to specific
/// [`SignatureSchema`].
///
/// For usage, see [`Wallet`](crate::Wallet).
#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait WalletSigner<S: SignatureSchema>: Sync {
    /// Signature error
    type Error: Debug + Display;

    /// Returns public key of the signer.
    fn public_key(&self) -> S::PublicKey;

    /// Sign [`RequestMessage`] according to [`SignatureSchema`]
    /// and return a proof serialized to string ready to be submitted to
    /// [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed) contract method
    async fn sign_wallet_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}

#[async_trait]
pub(crate) trait DynWalletSigner<S: SignatureSchema>: Send + Sync {
    fn dyn_public_key(&self) -> S::PublicKey;

    async fn dyn_sign_request_msg(
        &self,
        msg: &RequestMessage,
    ) -> Result<Proof, Box<dyn StdError + Send + Sync>>;
}

#[async_trait]
impl<SS, S> DynWalletSigner<SS> for S
where
    SS: SignatureSchema,
    S: WalletSigner<SS>,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    #[inline]
    fn dyn_public_key(&self) -> SS::PublicKey {
        self.public_key()
    }

    async fn dyn_sign_request_msg(
        &self,
        msg: &RequestMessage,
    ) -> Result<Proof, Box<dyn StdError + Send + Sync>> {
        self.sign_wallet_msg(msg).await.map_err(Into::into)
    }
}

impl<S> WalletSigner<S> for dyn DynWalletSigner<S> + '_
where
    S: SignatureSchema,
{
    type Error = Box<dyn StdError + Send + Sync>;

    #[inline]
    fn public_key(&self) -> S::PublicKey {
        self.dyn_public_key()
    }

    async fn sign_wallet_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        self.dyn_sign_request_msg(msg).await
    }
}
