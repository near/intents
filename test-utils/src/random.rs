use rand_chacha::{ChaChaRng, rand_core::RngCore};
pub use randomness::{self, CryptoRng, Rng, SeedableRng, seq::IteratorRandom};
use rstest::fixture;
use std::{num::ParseIntError, str::FromStr};

#[derive(Debug, Copy, Clone)]
pub struct Seed(pub u64);

impl Seed {
    #[must_use]
    pub fn from_entropy() -> Self {
        Seed(randomness::make_true_rng().next_u64())
    }

    #[must_use]
    pub fn from_entropy_and_print(test_name: &str) -> Self {
        let result = Seed(randomness::make_true_rng().next_u64());
        result.print_with_decoration(test_name);
        result
    }

    #[must_use]
    pub fn from_u64(v: u64) -> Self {
        Seed(v)
    }

    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn print_with_decoration(&self, test_name: &str) {
        println!("{test_name} seed: {}", self.0);
    }
}

impl FromStr for Seed {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.parse::<u64>()?;
        Ok(Seed::from_u64(v))
    }
}

impl From<u64> for Seed {
    fn from(v: u64) -> Self {
        Seed::from_u64(v)
    }
}

impl randomness::distributions::Distribution<Seed> for randomness::distributions::StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Seed {
        let new_seed = rng.next_u64();
        Seed::from_u64(new_seed)
    }
}

#[derive(Debug, Clone)]
pub struct TestRng(rand_chacha::ChaChaRng);

impl TestRng {
    #[must_use]
    pub fn new(seed: Seed) -> Self {
        Self(ChaChaRng::seed_from_u64(seed.as_u64()))
    }

    #[must_use]
    pub fn random(rng: &mut (impl Rng + CryptoRng)) -> Self {
        Self::new(Seed(rng.next_u64()))
    }
    #[must_use]
    pub fn from_entropy() -> Self {
        Self::new(Seed::from_entropy())
    }
}

impl RngCore for TestRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }
}

impl CryptoRng for TestRng {}

#[must_use]
pub fn make_seedable_rng(seed: Seed) -> impl Rng + CryptoRng {
    TestRng::new(seed)
}

pub fn gen_random_bytes(rng: &mut impl Rng, min_len: usize, max_len: usize) -> Vec<u8> {
    let data_length = rng.random_range(min_len..=max_len);
    let mut bytes = vec![0; data_length];
    rng.fill_bytes(&mut bytes);
    bytes
}

#[fixture]
pub fn random_seed() -> Seed {
    Seed::from_entropy()
}
