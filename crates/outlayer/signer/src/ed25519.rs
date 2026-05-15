pub use defuse_outlayer_kdf::ed25519::*;

use defuse_outlayer_kdf::{Curve, DerivationSchema, DeriveSigner, SchemaFn};
use sha3::{Digest, Sha3_256};

use crate::InMemorySigner;

pub const SCHEMA: SchemaFn<Ed25519, fn([u8; 32]) -> Scalar> = SchemaFn::new(|path| {
    const HASH_PREFIX: &[u8] = b"outlayer/ed25519/derive-tweak/v1";

    let path: [u8; 32] = Sha3_256::new_with_prefix(HASH_PREFIX)
        .chain_update(path)
        .finalize()
        .into();

    Scalar::from_bytes_mod_order(path)
});

impl DerivationSchema<Ed25519, [u8; 32]> for InMemorySigner {
    type Output = Scalar;

    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        SCHEMA.derive_path(path)
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
