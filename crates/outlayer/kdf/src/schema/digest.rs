use std::marker::PhantomData;

use digest::Output;

use crate::{DerivableCurve, DerivationSchema, DeriveSigner, Identity};

pub struct Digest<D, S = Identity> {
    next: S,
    _phantom: PhantomData<D>,
}

impl<C, D, S, P> DerivationSchema<C, P> for Digest<D, S>
where
    C: DerivableCurve,
    D: digest::Digest,
    P: AsRef<[u8]>,
    S: DerivationSchema<C, Output<D>>,
{
    type Output = S::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.next.derive_path(D::digest(path))
    }
}

impl<C, D, S, P> DeriveSigner<C, P> for Digest<D, S>
where
    C: DerivableCurve,
    D: digest::Digest,
    P: AsRef<[u8]>,
    S: DeriveSigner<C, Output<D>>,
{
    fn public_key(&self) -> C::PublicKey {
        self.next.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.next.derive_sign(D::digest(path), msg)
    }
}
