use borsh::{BorshSerialize, io};
#[cfg(feature = "digest")]
pub use digest_io::IoWrapper;
use impl_tools::autoimpl;

use crate::Schema;

/// [Borsh](borsh)-serialization adapter for [`Schema`]
///
/// # Example
///
/// ```rust
/// # use defuse_kdf::{borsh::Borsh, Schema};
/// let schema = Borsh::<Vec<u8>>::default();
/// let data = b"abc";
/// let derived = schema.derive_path(data);
/// assert_eq!(derived, [97, 98, 99]);
/// ```
#[autoimpl(Debug, Clone, Default where W::Data: trait)]
#[derive(Copy)]
pub struct Borsh<W: WriteFinalizer = Vec<u8>>(W::Data);

impl<W: WriteFinalizer> Borsh<W> {
    #[inline]
    pub const fn new(data: W::Data) -> Self {
        Self(data)
    }
}

impl<P, W> Schema<P> for Borsh<W>
where
    P: BorshSerialize,
    W: WriteFinalizer,
{
    type Output = W::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        let mut w = W::new(self.0.clone()); // branch a new writer
        borsh::to_writer(&mut w, &path).expect("borsh");
        w.finalize()
    }
}

/// Custom writer that can be used to avoid unnecessary allocations during
/// serialization when possible (e.g. serializing directly to hasher)
pub trait WriteFinalizer: io::Write {
    /// Preprocessed data to store between invocations
    type Data: Clone;
    /// Extracted output after serialization
    type Output;

    /// Initialize writer from pre-processed data
    fn new(data: Self::Data) -> Self;

    /// Finalize and extract output after serialization
    fn finalize(self) -> Self::Output;
}

/// Default implementation for serializing into byte vector
impl WriteFinalizer for Vec<u8> {
    type Data = ();
    type Output = Self;

    #[inline]
    fn new(_data: Self::Data) -> Self {
        Self::new()
    }

    #[inline]
    fn finalize(self) -> Self::Output {
        self
    }
}

#[cfg(feature = "digest")]
const _: () = {
    use digest::{Digest, Update, array::ArraySize, common::OutputSize};

    /// Optimized writer implementation to serialize directly to hasher
    impl<D> WriteFinalizer for IoWrapper<D>
    where
        D: Update + Digest + Clone,
    {
        type Data = D;

        /// `[u8; N]`
        type Output = <OutputSize<D> as ArraySize>::ArrayType<u8>;

        #[inline]
        fn new(data: Self::Data) -> Self {
            Self(data)
        }

        #[inline]
        fn finalize(self) -> Self::Output {
            self.0.finalize().into()
        }
    }
};
