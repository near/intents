use crate::Schema;

/// Hex-encoding adaptor for [`Schema`]
///
/// ```rust
/// use defuse_kdf::{hex::Hex, Schema};
///
/// assert_eq!(Hex.derive_path(b"(=_=)"), "283d5f3d29")
/// ```
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
