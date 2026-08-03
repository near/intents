use std::error::Error as StdError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

/// An error returned from [`Wallet`] methods
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "near-kit")]
    #[error(transparent)]
    Near(#[from] ::near_kit::Error),

    /// An error occurred during [relaying](WalletRelayer::relay_wallet_msg)
    /// signed [request](RequestMessage).
    #[error("relayer: {0}")]
    Relayer(Box<dyn StdError + Send + Sync>),

    /// An error occurred during [signing](WalletSigner::sign_wallet_msg)
    /// wallet [request](RequestMessage).
    #[error("signer: {0}")]
    Signer(Box<dyn StdError + Send + Sync>),
}

#[cfg(feature = "near-kit")]
impl From<::near_kit::RpcError> for Error {
    #[inline]
    fn from(err: ::near_kit::RpcError) -> Self {
        Self::Near(err.into())
    }
}
