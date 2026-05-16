use borsh::{BorshSerialize, io};

use crate::DerivationSchema;

// TODO: docs
#[derive(Default)]
pub struct Borsh<W: WriteExtract = Vec<u8>>(W::Store);

impl<W: WriteExtract> Borsh<W> {
    #[inline]
    pub const fn new(store: W::Store) -> Self {
        Self(store)
    }
}

impl<P, W> DerivationSchema<P> for Borsh<W>
where
    P: BorshSerialize,
    W: WriteExtract,
{
    type Output = W::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        let mut w = W::new(self.0.clone());

        borsh::to_writer(&mut w, &path).expect("borsh");

        w.finalize()
    }
}

// TODO: naming
pub trait WriteExtract: io::Write {
    type Store: Clone;
    type Output;

    fn new(store: Self::Store) -> Self;

    fn finalize(self) -> Self::Output;
}

impl WriteExtract for Vec<u8> {
    type Store = ();
    type Output = Self;

    fn new(_: Self::Store) -> Self {
        Vec::new()
    }

    fn finalize(self) -> Self::Output {
        self
    }
}

#[cfg(feature = "digest")]
const _: () = {
    use ::digest::Output;
    use digest::{Digest, Update};
    use digest_io::IoWrapper;

    impl<D> WriteExtract for IoWrapper<D>
    where
        D: Update + Digest + Clone,
    {
        type Store = D;

        type Output = Output<D>;

        fn new(store: Self::Store) -> Self {
            IoWrapper(store)
        }

        fn finalize(self) -> Self::Output {
            self.0.finalize()
        }
    }
};
