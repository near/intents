pub use defuse_outlayer_kdf::secp256k1::*;

use defuse_outlayer_kdf::{
    Curve, DerivationSchema, DeriveSigner, SchemaFn,
    secp256k1::k256::{U256, elliptic_curve::ops::Reduce},
};
use sha3::{Digest, Sha3_256};

use crate::InMemorySigner;

pub const SCHEMA: SchemaFn<Secp256k1, fn([u8; 32]) -> NonZeroScalar> = SchemaFn::new(|path| {
    const HASH_PREFIX: &[u8] = b"outlayer/secp256k1/derive-tweak/v1";

    let path: [u8; 32] = Sha3_256::new_with_prefix(HASH_PREFIX)
        .chain_update(path)
        .finalize()
        .into();

    Reduce::<U256>::reduce_bytes(&path.into())
});

impl DerivationSchema<Secp256k1, [u8; 32]> for InMemorySigner {
    type Output = NonZeroScalar;

    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        SCHEMA.derive_path(path)
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
