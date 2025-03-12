pub use rand::prelude::SliceRandom;
pub use rand::{CryptoRng, Rng, RngCore, SeedableRng, seq};

pub mod distributions {
    pub use rand::distr::{Alphanumeric, Distribution, StandardUniform, weighted::WeightedIndex};
    pub mod uniform {
        pub use rand::distr::uniform::SampleRange;
    }
}

pub mod rngs {
    pub use rand::rngs::OsRng;
    pub use rand::rngs::mock::StepRng;
}

#[must_use]
pub fn make_true_rng() -> impl Rng + CryptoRng {
    rand::rngs::StdRng::from_os_rng()
}

#[must_use]
pub fn make_pseudo_rng() -> impl Rng {
    rand::rngs::ThreadRng::default()
}
