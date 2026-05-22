#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

use core::marker::PhantomData;

use defuse_kdf::{CurveArithmetic, Derive, DeriveExt, Schema, digest::Digest};
use impl_tools::autoimpl;
use near_account_id::AccountIdRef;
use sha3::{Digest as _, Sha3_256};

use crate::derive_from_path;

pub fn derive_scalar<C>(
    predecessor_id: impl AsRef<AccountIdRef>,
) -> Derive<ToScalar<C>, Digest<Sha3_256>>
where
    C: NearMpcCurve,
{
    ToScalar::<C>::new().derive(derive_tweak(predecessor_id))
}

fn derive_tweak(predecessor_id: impl AsRef<AccountIdRef>) -> Digest<Sha3_256> {
    // See <https://github.com/near/mpc/blob/f07b9145b17e2372be768aa67a2106be9989a7d7/crates/near-mpc-crypto-types/src/kdf.rs#L6-L13>
    const TWEAK_DERIVATION_PREFIX: &str = "near-mpc-recovery v0.1.0 epsilon derivation:";

    thread_local! {
        // per-thread lazily-initialized hasher with pre-processed prefix
        static HASHER: Sha3_256 = Sha3_256::new_with_prefix(TWEAK_DERIVATION_PREFIX);
    }

    derive_from_path(HASHER.with(Clone::clone), predecessor_id)
}

#[autoimpl(Debug, Clone, Copy, Default)]
pub struct ToScalar<C>(PhantomData<C>);

impl<C> ToScalar<C> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<C> Schema<[u8; 32]> for ToScalar<C>
where
    C: NearMpcCurve,
{
    type Output = C::Scalar;

    fn derive_path(&self, tweak: [u8; 32]) -> Self::Output {
        C::to_scalar(tweak)
    }
}

pub trait NearMpcCurve: CurveArithmetic + sealed::Sealed {
    fn to_scalar(tweak: [u8; 32]) -> Self::Scalar;
}

mod sealed {
    pub trait Sealed {}
}
