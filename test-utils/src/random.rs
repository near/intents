use ::arbitrary::{Arbitrary, Unstructured};
pub use defuse_randomness::{self as randomness, CryptoRng, Rng, SeedableRng, seq::IteratorRandom};
use rand_chacha::{ChaChaRng, rand_core::RngCore};
use rstest::fixture;
use std::{fmt::Display, num::ParseIntError, ops::RangeBounds, str::FromStr};

#[derive(Debug, Copy, Clone)]
pub struct Seed(pub u64);

impl Seed {
    #[must_use]
    pub fn from_entropy() -> Self {
        Self(randomness::make_true_rng().next_u64())
    }

    #[must_use]
    pub fn from_entropy_and_print(test_name: &str) -> Self {
        let result = Self(randomness::make_true_rng().next_u64());
        result.print_with_decoration(test_name);
        result
    }

    #[must_use]
    pub const fn from_u64(v: u64) -> Self {
        Self(v)
    }

    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn print_with_decoration(&self, test_name: &str) {
        println!("{test_name} seed: {}", self.0);
    }

    #[must_use]
    pub fn derive_seed(&self) -> Self {
        let mut rng = rng(*self);
        rng.random()
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for Seed {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.parse::<u64>()?;
        Ok(Self::from_u64(v))
    }
}

impl From<u64> for Seed {
    fn from(v: u64) -> Self {
        Self::from_u64(v)
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

fn range_to_random_size(rng: &mut impl Rng, size: impl RangeBounds<usize>) -> usize {
    let start = match size.start_bound() {
        std::ops::Bound::Included(&n) => n,
        std::ops::Bound::Excluded(&n) => n + 1,
        std::ops::Bound::Unbounded => 0,
    };
    let end = match size.end_bound() {
        std::ops::Bound::Included(&n) => n + 1,
        std::ops::Bound::Excluded(&n) => n,
        std::ops::Bound::Unbounded => usize::MAX,
    };
    rng.random_range(start..end)
}

pub fn gen_random_string<R: Rng>(rng: &mut R, size: impl RangeBounds<usize>) -> String {
    let size = range_to_random_size(rng, size);
    rng.sample_iter(&randomness::distributions::Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

#[fixture]
pub fn random_seed() -> Seed {
    let seed = Seed::from_entropy();
    eprintln!("======= SEED =======\n{seed}\n====================",);
    seed
}

#[fixture]
#[must_use]
pub fn rng(random_seed: Seed) -> impl Rng + CryptoRng {
    TestRng::new(random_seed)
}

#[fixture]
pub fn random_bytes<'a>(
    #[default(50..1000)] size: impl RangeBounds<usize>,
    mut rng: impl Rng,
) -> Vec<u8> {
    let data_length = range_to_random_size(&mut rng, size);
    let mut bytes = vec![0; data_length];
    rng.fill_bytes(&mut bytes);
    bytes
}

#[fixture]
pub fn make_arbitrary<T>(random_bytes: Vec<u8>) -> T
where
    for<'a> T: Arbitrary<'a>,
{
    let mut u = Unstructured::new(&random_bytes);
    u.arbitrary().unwrap()
}
