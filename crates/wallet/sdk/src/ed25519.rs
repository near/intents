use core::convert::Infallible;

use defuse_wallet::signature::{
    RequestMessage, WALLET_DOMAIN,
    ed25519::{Ed25519PublicKey, Ed25519Signature},
};
pub use ed25519_dalek::SigningKey;
use ed25519_dalek::ed25519::signature::Signer as Ed25519Signer;
use near_sdk::{borsh, env};

use crate::{Proof, Signer};

impl Signer for SigningKey {
    type PublicKey = Ed25519PublicKey;
    type Error = Infallible;

    fn public_key(&self) -> Self::PublicKey {
        Ed25519PublicKey(self.verifying_key().to_bytes())
    }

    fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let serialized = borsh::to_vec(msg).unwrap_or_else(|_| unreachable!());
        let hash = env::sha256_array(&[WALLET_DOMAIN, &serialized].concat());
        let signature = <Self as Ed25519Signer<_>>::sign(self, &hash).to_bytes();
        Ok(Ed25519Signature(signature).to_string())
    }
}
