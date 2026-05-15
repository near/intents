#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

use std::borrow::Cow;

use defuse_outlayer_kdf::{DerivableCurve, DerivationSchema};
use near_account_id::AccountIdRef;
use near_mpc_crypto_types::{Tweak, kdf::derive_tweak};

pub struct NearMpcDerivation<'a> {
    predecessor_id: Cow<'a, AccountIdRef>,
}

impl<'a> NearMpcDerivation<'a> {
    pub fn new(predecessor_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            predecessor_id: predecessor_id.into(),
        }
    }
}

impl<C, P> DerivationSchema<C, P> for NearMpcDerivation<'_>
where
    C: NearMpcCurve,
    P: AsRef<str>,
{
    type Output = C::Tweak;

    /// See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/lib.rs#L411-L449>
    fn derive_path(&self, path: P) -> Self::Output {
        let tweak = derive_tweak(&self.predecessor_id.clone().into_owned(), path.as_ref());

        C::tweak(tweak)
    }
}

pub trait NearMpcCurve: DerivableCurve + sealed::Sealed {
    fn tweak(tweak: Tweak) -> Self::Tweak;
}

mod sealed {
    pub trait Sealed {}
}
