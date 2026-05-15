use digest::Output;

use crate::{DerivableCurve, DerivationSchema};

#[derive(Default, Clone)]
pub struct Digest<D>(D);

impl<D> Digest<D>
where
    D: digest::Digest,
{
    pub const fn new(digest: D) -> Self {
        Self(digest)
    }

    pub fn new_with_prefix(data: impl AsRef<[u8]>) -> Self {
        Self::new(D::new_with_prefix(data))
    }
}

impl<C, D, P> DerivationSchema<C, P> for Digest<D>
where
    C: DerivableCurve,
    D: digest::Digest + Clone,
    P: AsRef<[u8]>,
{
    type Output = Output<D>;

    fn derive_path(&self, path: P) -> Self::Output {
        self.0.clone().chain_update(path).finalize()
    }
}

// TODO
pub trait Prefix<C: DerivableCurve> {}
