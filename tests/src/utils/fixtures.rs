use arbitrary_with::{Arbitrary, Unstructured};
use defuse::core::{Nonce, crypto::PublicKey};
use defuse_test_utils::random::{Rng, rng};
use rstest::fixture;

#[fixture]
pub fn nonce(mut rng: impl Rng) -> Nonce {
    let mut random_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}

#[fixture]
pub fn public_key(mut rng: impl Rng) -> PublicKey {
    let mut random_bytes = [0u8; 64];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}

#[fixture]
pub fn signing_standard<T>(mut rng: impl Rng) -> T
where
    for<'a> T: Arbitrary<'a>,
{
    let mut random_bytes = [0u8; 8];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}
