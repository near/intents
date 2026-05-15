use crate::{DerivableCurve, DerivationSchema, Identity};

#[derive(Default)]
pub struct Hex<S = Identity>(S);

impl<S> Hex<S> {
    pub const fn new(next: S) -> Self {
        Self(next)
    }
}

impl<S, C, P> DerivationSchema<C, P> for Hex<S>
where
    C: DerivableCurve + ?Sized,
    P: AsRef<[u8]>,
    S: DerivationSchema<C, String>,
{
    type Output = S::Output;

    fn derive(&self, path: P) -> Self::Output {
        self.0.derive(hex::encode(path))
    }
}
