use digest::Output;

use crate::Schema;

/// Hashing adaptor for [`Schema`]
#[derive(Debug, Clone, Default)]
pub struct Digest<D>(D);

impl<D> Digest<D>
where
    D: digest::Digest,
{
    /// Create new with already created [`digest::Digest`] instance
    #[inline]
    pub const fn new(digest: D) -> Self {
        Self(digest)
    }
}

impl<D, P> Schema<P> for Digest<D>
where
    D: digest::Digest + Clone,
    P: AsRef<[u8]>,
{
    type Output = Output<D>;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        self.0.clone().chain_update(path).finalize()
    }
}
