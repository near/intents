use crate::{Result, Schema};

/// Hex-encoding adaptor for [`Schema`]
///
/// ```rust
/// use defuse_derivation_schema::{DerivationSchema, hex::Hex};
///
/// assert_eq!(Hex.derive(b"(=_=)"), "283d5f3d29")
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Hex;

impl<T> Schema<T> for Hex
where
    T: AsRef<[u8]>,
{
    type Output = String;

    #[inline]
    fn derive(&self, input: T) -> Result<Self::Output> {
        Ok(hex::encode(input))
    }
}

// TODO: pub trait Case; upper, lower
// TODO: optional prepend 0x?
