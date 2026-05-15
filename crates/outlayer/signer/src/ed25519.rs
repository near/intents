pub use defuse_outlayer_kdf::ed25519::*;

use defuse_outlayer_kdf::{Curve, DerivationSchema, DeriveSigner};

use crate::InMemorySigner;

impl DerivationSchema<Ed25519, [u8; 32]> for InMemorySigner {
    type Output = Scalar;

    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        // TODO: hash
        FromBytesModOrder.derive_path(path)
    }
}

impl DeriveSigner<Ed25519, [u8; 32]> for InMemorySigner {
    fn public_key(&self) -> <Ed25519 as Curve>::PublicKey {
        self.ed25519_master_sk.verifying_key()
    }

    fn derive_sign(&self, path: [u8; 32], msg: &[u8]) -> <Ed25519 as Curve>::Signature {
        let tweak = DerivationSchema::<Ed25519, _>::derive_path(self, path);

        self.ed25519_master_sk.derive_sign(tweak, msg)
    }
}
