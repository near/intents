use crate::{DerivableCurve, DerivationSchema, DeriveSigner, Identity};

// TODO: remove identity
#[derive(Default)]
pub struct Hex<S = Identity>(S);

impl<S> Hex<S> {
    pub const fn new(next: S) -> Self {
        Self(next)
    }
}

impl<S, C, P> DerivationSchema<C, P> for Hex<S>
where
    C: DerivableCurve,
    P: AsRef<[u8]>,
    S: DerivationSchema<C, String>,
{
    type Output = S::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.0.derive_path(hex::encode(path))
    }
}

impl<C, P, S> DeriveSigner<C, P> for Hex<S>
where
    C: DerivableCurve,
    P: AsRef<[u8]>,
    S: DeriveSigner<C, String>,
{
    fn public_key(&self) -> C::PublicKey {
        self.0.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.0.derive_sign(hex::encode(path), msg)
    }
}
