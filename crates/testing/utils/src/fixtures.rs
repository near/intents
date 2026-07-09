use arbitrary::Unstructured;
use defuse_core::PublicKey;
use defuse_crypto::{
    ed25519::Ed25519PublicKey, p256::P256UncompressedPublicKey,
    secp256k1::Secp256k1UncompressedPublicKey,
};
use rstest::fixture;

use super::random::{Rng, RngExt, rng};

#[fixture]
pub fn public_key(mut rng: impl Rng) -> PublicKey {
    let mut random_bytes = [0u8; 64];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}

#[fixture]
pub fn ed25519_pk(mut rng: impl Rng) -> PublicKey {
    Ed25519PublicKey(rng.random()).into()
}

#[fixture]
pub fn secp256k1_pk(mut rng: impl Rng) -> PublicKey {
    Secp256k1UncompressedPublicKey(rng.random()).into()
}

#[fixture]
pub fn p256_pk(mut rng: impl Rng) -> PublicKey {
    P256UncompressedPublicKey(rng.random()).into()
}
