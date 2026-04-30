mod ed25519;
mod secp256k1;

use defuse_outlayer_primitives::crypto::DerivationPath;

use crate::State;

pub use ed25519::Ed25519Host;
pub use secp256k1::Secp256k1Host;

impl State<'_> {
    fn tweak(&self, path: impl AsRef<str>) -> [u8; 32] {
        let path = DerivationPath {
            app_id: self.ctx.app_id.as_ref(),
            path: path.as_ref().into(),
        };

        path.hash()
    }
}

/// Trait defining crypto-related host functions available to the component
pub trait CryptoHost: Ed25519Host + Secp256k1Host + Send {}

impl<T: Ed25519Host + Secp256k1Host + Send> CryptoHost for T {}
