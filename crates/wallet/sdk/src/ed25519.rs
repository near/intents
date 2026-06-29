use core::convert::Infallible;

use defuse_crypto::{Ed25519PublicKey, Ed25519Signature};

use defuse_wallet_core::RequestMessage;
use ed25519_dalek::ed25519::signature::Signer as Ed25519Signer;
pub use ed25519_dalek::{self, SigningKey};

use crate::{Proof, Signer};

impl Signer for SigningKey {
    type PublicKey = Ed25519PublicKey;
    type Error = Infallible;

    fn public_key(&self) -> Self::PublicKey {
        Ed25519PublicKey(self.verifying_key().to_bytes())
    }

    fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let signature = <Self as Ed25519Signer<_>>::sign(self, &msg.hash()).to_bytes();
        Ok(Ed25519Signature(signature).to_string())
    }
}
