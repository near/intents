#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

use std::{borrow::Cow, marker::PhantomData};

use defuse_kdf::{CurveArithmetics, Schema};
use near_account_id::AccountIdRef;
use near_mpc_crypto_types::{Tweak, kdf::derive_tweak};

#[derive(Debug, Clone)]
pub struct NearMpcDerivation<'a, C> {
    predecessor_id: Cow<'a, AccountIdRef>,
    _curve: PhantomData<C>,
}

impl<'a, C> NearMpcDerivation<'a, C> {
    pub fn new(predecessor_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            predecessor_id: predecessor_id.into(),
            _curve: PhantomData,
        }
    }
}

impl<C, P> Schema<P> for NearMpcDerivation<'_, C>
where
    C: NearMpcCurve,
    P: AsRef<str>,
{
    type Output = C::Scalar;

    /// See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/lib.rs#L411-L449>
    fn derive_path(&self, path: P) -> Self::Output {
        let tweak = derive_tweak(&self.predecessor_id.clone().into_owned(), path.as_ref());

        C::tweak(tweak)
    }
}

pub trait NearMpcCurve: CurveArithmetics + sealed::Sealed {
    fn tweak(tweak: Tweak) -> Self::Scalar;
}

mod sealed {
    pub trait Sealed {}
}
