pub use defuse_outlayer_kdf::secp256k1::*;

use defuse_outlayer_kdf::{Curve, DerivationSchema, DeriveSigner};

use crate::InMemorySigner;

impl DerivationSchema<Secp256k1, [u8; 32]> for InMemorySigner {
    type Output = NonZeroScalar;

    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Reduce.derive_path(path)
    }
}

impl DeriveSigner<Secp256k1, [u8; 32]> for InMemorySigner {
    fn public_key(&self) -> <Secp256k1 as Curve>::PublicKey {
        *self.secp256k1_master_sk.verifying_key()
    }

    fn derive_sign(&self, path: [u8; 32], msg: &[u8; 32]) -> <Secp256k1 as Curve>::Signature {
        let tweak = DerivationSchema::<Secp256k1, _>::derive_path(self, path);
        self.secp256k1_master_sk.derive_sign(tweak, msg)
    }
}
