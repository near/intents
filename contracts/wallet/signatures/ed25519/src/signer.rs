use defuse_crypto::{
    Signer,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_wallet_sdk::{Proof, RequestMessage, WalletSigner};

use crate::WalletEd25519;

/// Signer wrapper for [`WalletEd25519`] signature schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::AsRef)]
pub struct WalletEd25519Signer<S>(pub S);

impl<S> WalletSigner<WalletEd25519> for WalletEd25519Signer<S>
where
    S: Signer<Ed25519>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> Ed25519PublicKey {
        self.0.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let sig = self.0.sign(&msg.hash()).await?;

        Ok(Ed25519Signature::from(sig).to_string())
    }
}
