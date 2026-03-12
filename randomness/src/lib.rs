use rand::rngs::StdRng;

pub use rand::*;

#[must_use]
pub fn make_true_rng() -> impl CryptoRng {
    make_rng::<StdRng>()
}

#[must_use]
pub fn make_pseudo_rng() -> impl Rng {
    rng()
}
