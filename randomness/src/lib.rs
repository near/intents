pub use rand::prelude::SliceRandom;
pub use rand::{seq, CryptoRng, Rng, RngCore, SeedableRng};

pub mod distributions {
    pub use rand::distr::{weighted::WeightedIndex, Alphanumeric, Distribution, StandardUniform};
    pub mod uniform {
        pub use rand::distr::uniform::SampleRange;
    }
}

pub mod rngs {
    pub use rand::rngs::mock::StepRng;
    pub use rand::rngs::OsRng;
}

#[must_use]
pub fn make_true_rng() -> impl Rng + CryptoRng {
    rand::rngs::StdRng::from_os_rng()
}

#[must_use]
pub fn make_pseudo_rng() -> impl Rng {
    rand::rngs::ThreadRng::default()
}
