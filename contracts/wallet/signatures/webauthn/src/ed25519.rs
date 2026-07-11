pub use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
pub use defuse_webauthn::ed25519::Ed25519;

use crate::WalletWebauthnAlgorithm;

impl WalletWebauthnAlgorithm for Ed25519 {
    type PublicKey = Ed25519PublicKey;
    type Signature = Ed25519Signature;
}

// TODO: tests
