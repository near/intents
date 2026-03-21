use defuse_wallet::signature::{
    RequestMessage, WALLET_DOMAIN,
    ed25519::{Ed25519PublicKey, Ed25519Signature},
};
pub use ed25519_dalek;
use ed25519_dalek::{SigningKey, ed25519::signature::Signer as Ed25519Signer};
use near_sdk::{borsh, env::sha256_array};

use crate::{Signature, Signer};

impl Signer for SigningKey {
    type PublicKey = Ed25519PublicKey;

    fn public_key(&self) -> Self::PublicKey {
        Ed25519PublicKey(self.verifying_key().to_bytes())
    }

    fn sign(&self, msg: &RequestMessage) -> Signature {
        let serialized = borsh::to_vec(msg).unwrap();
        let hash = sha256_array(&[WALLET_DOMAIN, &serialized].concat());
        let signature = <Self as Ed25519Signer<_>>::sign(self, &hash).to_bytes();
        Ed25519Signature(signature).to_string()
    }
}
