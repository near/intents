use crate::Schema;

/// Hex-encoding adaptor for [`Schema`]
#[derive(Debug, Clone, Copy, Default)]
pub struct Hex;

impl<P> Schema<P> for Hex
where
    P: AsRef<[u8]>,
{
    type Output = String;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        hex::encode(path)
    }
}
