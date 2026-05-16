use std::marker::PhantomData;

use defuse_outlayer_kdf::{DerivableCurve, DerivationSchema};
use sha3::{Digest, Sha3_256};

// TODO: maybe expose as combination of Then<Digest>, etc...?
// TODO: docs
pub struct Schema<C>(PhantomData<C>);

impl<C, P> DerivationSchema<P> for Schema<C>
where
    C: DomainCurve,
    P: AsRef<[u8]>,
{
    type Output = C::Tweak;

    fn derive_path(&self, path: P) -> Self::Output {
        let path: [u8; 32] = Sha3_256::new_with_prefix(C::DOMAIN_SEPARATOR)
            .chain_update(path)
            .finalize()
            .into();

        C::ToTweak::default().derive_path(path)
    }
}

impl<C> Default for Schema<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

// TODO: docs
pub trait DomainCurve: DerivableCurve + sealed::Sealed {
    /// Domain separator to avoid algebraic relations between derived keys
    const DOMAIN_SEPARATOR: &[u8];

    // TODO
    type ToTweak: DerivationSchema<[u8; 32], Output = Self::Tweak> + Default;
}

pub(crate) mod sealed {
    pub trait Sealed {}
}
