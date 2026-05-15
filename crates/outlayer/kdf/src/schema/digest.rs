use std::marker::PhantomData;

use digest::Output;

use crate::{DerivableCurve, DerivationSchema};

pub struct Digest<D>(PhantomData<D>);

impl<C, D, P> DerivationSchema<C, P> for Digest<D>
where
    C: DerivableCurve,
    D: digest::Digest,
    P: AsRef<[u8]>,
{
    type Output = Output<D>;

    fn derive_path(&self, path: P) -> Self::Output {
        D::digest(path)
    }
}

impl<D> Default for Digest<D> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

// TODO
pub trait Prefix<C: DerivableCurve> {}
