use defuse_crypto::{
    Signer,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_wallet::{RequestMessage, offchain::OffchainMessage};
use defuse_wallet_sdk::{Proof, WalletSigner};

use crate::{MessageError, WalletSolana, offchain_signing_message, request_signing_message};

/// Adapter for an Ed25519 signer that signs full message bytes without prehashing.
#[derive(Debug, Clone, Copy)]
pub struct WalletSolanaSigner<S>(pub S);

/// Rendering or underlying signer failure.
#[derive(Debug, thiserror::Error)]
pub enum SignError<E> {
    #[error(transparent)]
    Message(#[from] MessageError),
    #[error("signer: {0}")]
    Signer(E),
}

impl<S: Signer<Ed25519>> WalletSolanaSigner<S> {
    async fn sign_message(&self, message: String) -> Result<Proof, SignError<S::Error>> {
        let signature = self
            .0
            .sign(message.as_bytes())
            .await
            .map_err(SignError::Signer)?;

        Ok(Ed25519Signature::from(signature).to_string())
    }
}

impl<S: Signer<Ed25519>> WalletSigner<WalletSolana> for WalletSolanaSigner<S> {
    type Error = SignError<S::Error>;

    fn public_key(&self) -> Ed25519PublicKey {
        self.0.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        self.sign_message(request_signing_message(&self.public_key(), msg)?)
            .await
    }

    async fn sign_offchain_msg(&self, msg: &OffchainMessage) -> Result<Proof, Self::Error> {
        self.sign_message(offchain_signing_message(&self.public_key(), msg)?)
            .await
    }
}
