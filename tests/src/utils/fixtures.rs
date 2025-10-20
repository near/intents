use arbitrary_with::{Arbitrary, Unstructured};
use chrono::{TimeDelta, Utc};
use defuse::core::{
    Deadline, ExpirableNonce, Nonce, Salt, SaltedNonce, VersionedNonce, crypto::PublicKey,
};
use defuse_test_utils::random::{Rng, rng};
use rstest::fixture;

#[fixture]
pub fn nonce(mut rng: impl Rng) -> Nonce {
    let mut random_bytes = [0u8; 64];
    rng.fill_bytes(&mut random_bytes);
    let mut u = Unstructured::new(&random_bytes);
    let nonce_bytes: [u8; 15] = u.arbitrary().unwrap();
    let current_timestamp = Utc::now();
    let deadline = Deadline::new(
        current_timestamp
            .checked_add_signed(TimeDelta::days(1))
            .unwrap(),
    );
    let salt: Salt = Salt::derive(0);
    let salted = SaltedNonce::new(salt, ExpirableNonce::new(deadline, nonce_bytes));
    VersionedNonce::V1(salted).into()
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
