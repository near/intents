use std::marker::PhantomData;

use defuse_kdf::{DerivableCurve, DerivationSchema, Reduce};
use digest::{Digest, OutputSizeUser, array::ArraySize};

// TODO: maybe expose as combination of Then<Digest>, etc...?
// TODO: docs
#[derive(Copy)]
pub struct Schema<C>(PhantomData<C>);

impl<C, P> DerivationSchema<P> for Schema<C>
where
    C: DomainCurve,
    // scalar is reducable from digest output converted to array
    Reduce<C>: DerivationSchema<
            <<C::Digest as OutputSizeUser>::OutputSize as ArraySize>::ArrayType<u8>,
            Output = C::Tweak,
        >,
    P: AsRef<[u8]>,
{
    type Output = C::Tweak;

    fn derive_path(&self, path: P) -> Self::Output {
        // use domain-separated hashers to avoid algebraic relations between
        // derived keys
        let path = C::domain_hasher().chain_update(path).finalize().into();

        // reduce
        Reduce::<C>::default().derive_path(path)
    }
}

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

// TODO: docs
///
pub trait DomainCurve: DerivableCurve + sealed::Sealed {
    type Digest: Digest;

    /// Returns a hasher with already processed domain separator
    fn domain_hasher() -> Self::Digest;
}

pub(crate) mod sealed {
    pub trait Sealed {}
}
