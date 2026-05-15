use crate::{DerivableCurve, DerivationSchema};

#[derive(Default)]
pub struct Hex;

impl<C, P> DerivationSchema<C, P> for Hex
where
    C: DerivableCurve,
    P: AsRef<[u8]>,
{
    type Output = String;

    fn derive_path(&self, path: P) -> Self::Output {
        hex::encode(path)
    }
}
