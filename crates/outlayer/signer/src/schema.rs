use std::marker::PhantomData;

/// [`DerivationSchema`](defuse_kdf::DerivationSchema) used by the signer
#[derive(Copy)]
pub struct Schema<C>(PhantomData<C>);

impl<C> Default for Schema<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C> Clone for Schema<C> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
