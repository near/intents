use std::marker::PhantomData;

use digest::Output;

use crate::{DerivableCurve, DerivationSchema, Identity};

pub struct Digest<D, S = Identity> {
    next: S,
    _phantom: PhantomData<D>,
}

impl<C, D, S, P> DerivationSchema<C, P> for Digest<D, S>
where
    C: DerivableCurve + ?Sized,
    D: digest::Digest,
    P: AsRef<[u8]>,
    S: DerivationSchema<C, Output<D>>,
{
    type Output = S::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.next.derive_path(D::digest(path))
    }
}
