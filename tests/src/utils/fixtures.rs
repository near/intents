use arbitrary_with::Unstructured;
use defuse::core::crypto::PublicKey;
use defuse_test_utils::random::{Rng, rng};
use defuse::core::ton_connect::tlb_ton::MsgAddress;
use rstest::fixture;

#[fixture]
pub fn public_key(mut rng: impl Rng) -> PublicKey {
    let mut random_bytes = [0u8; 64];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}

#[fixture]
pub fn ed25519_pk(mut rng: impl Rng) -> PublicKey {
    PublicKey::Ed25519(rng.random())

}

#[fixture]
pub fn secp256k1_pk(mut rng: impl Rng) -> PublicKey {
    PublicKey::Secp256k1(rng.random())
}

#[fixture]
pub fn p256_pk(mut rng: impl Rng) -> PublicKey {
    PublicKey::P256(rng.random())
}

#[fixture]
pub fn msg_address(mut rng: impl Rng) -> MsgAddress {
    let mut random_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(& mut random_bytes);
    u.arbitrary().unwrap()
}

