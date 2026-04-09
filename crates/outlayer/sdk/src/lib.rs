use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutlayerError {
    #[error("Failed to get public key: {0}")]
    FailedToGetPublicKey(String),
    #[error("Failed to sign: {0}")]
    FailedToSign(String),
    #[error("Invalid public key length")]
    InvalidPublicKeyLength,
    #[error("Invalid signature length")]
    InvalidSignatureLength,
}

pub type OutlayerResult<T> = Result<T, OutlayerError>;

#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;
