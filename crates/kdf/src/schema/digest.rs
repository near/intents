use digest::Output;

use crate::DerivationSchema;

/// Hashing adaptor [`DerivationSchema`]
#[derive(Default, Clone)]
pub struct Digest<D>(D);

impl<D> Digest<D>
where
    D: digest::Digest,
{
    /// Create new with already created [`digest::Digest`] instance
    pub const fn new(digest: D) -> Self {
        Self(digest)
    }
}

impl<D, P> DerivationSchema<P> for Digest<D>
where
    D: digest::Digest + Clone,
    P: AsRef<[u8]>,
{
    type Output = Output<D>;

    fn derive_path(&self, path: P) -> Self::Output {
        self.0.clone().chain_update(path).finalize()
    }
}
