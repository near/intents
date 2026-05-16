use crate::DerivationSchema;

/// Hex-encoding adaptor for [`DerivationSchema`]
#[derive(Default, Clone)]
pub struct Hex;

impl<P> DerivationSchema<P> for Hex
where
    P: AsRef<[u8]>,
{
    type Output = String;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        hex::encode(path)
    }
}
