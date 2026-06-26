use digest::{array::ArraySize, common::OutputSize};

use crate::Schema;

/// Hashing adaptor for [`Schema`]
///
/// ```rust
/// use defuse_kdf::{digest::Digest, Schema};
/// # use hex_literal::hex;
/// use defuse_digest::sha3::Sha3_256;
///
/// let schema = Digest::<Sha3_256>::default();
/// assert_eq!(
///     schema.derive_path(b"test"),
///     hex!("36f028580bb02cc8272a9a020f4200e346e276ae664e45ee80745574e2f5ab80"),
/// );
/// ```
#[derive(Debug, Clone, Default)]
pub struct Digest<D>(D);

impl<D> Digest<D>
where
    D: ::digest::Digest,
{
    /// Create new schema which always re-uses given hasher by cloning it.
    ///
    /// ```rust
    /// use defuse_kdf::{digest::Digest, Schema};
    /// # use hex_literal::hex;
    /// use defuse_digest::{Digest as _, sha3::Sha3_256};
    ///
    /// let schema = Digest::new(Sha3_256::new_with_prefix(b"prefix"));
    /// assert_eq!(
    ///     schema.derive_path(b"test"),
    ///     hex!("c71179eae984b918c4a7736419745670d1a6fb81e441d703dcd76193a78c5007"),
    /// );
    /// ```
    #[inline]
    pub const fn new(hasher: D) -> Self {
        Self(hasher)
    }
}

impl<D, P> Schema<P> for Digest<D>
where
    D: ::digest::Digest + Clone,
    P: AsRef<[u8]>,
{
    /// `[u8; N]`
    type Output = <OutputSize<D> as ArraySize>::ArrayType<u8>;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        let hasher = self.0.clone(); // branch
        hasher.chain_update(path).finalize().into()
    }
}
