pub use defuse_crypto::ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature};

use crate::WalletNep413Curve;

impl WalletNep413Curve for Ed25519 {
    type StoredPublicKey = Ed25519PublicKey;
    type Proof = Ed25519Signature;
}
