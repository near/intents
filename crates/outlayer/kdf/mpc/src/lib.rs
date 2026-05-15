// #[cfg(feature = "ed25519")]
// mod ed25519;
// #[cfg(feature = "secp256k1")]
// mod secp256k1;

// use std::{borrow::Cow, marker::PhantomData};

// use defuse_outlayer_kdf::{DerivableCurve, DerivationSchema, DerivationScheme, Identity};
// use near_account_id::AccountId;
// use near_mpc_crypto_types::{Tweak, kdf::derive_tweak};

// pub struct NearMpc<S: ?Sized = Identity>(PhantomData<S>);

// // TODO: derives
// pub struct NearMpcDerivationPath<'a, P = Cow<'a, str>> {
//     pub predecessor_id: Cow<'a, AccountId>,
//     pub path: P,
// }

// impl<'a, C, S, P> DerivationScheme<C, NearMpcDerivationPath<'a, P>> for NearMpc<S>
// where
//     S: DerivationSchema<P> + ?Sized,
//     S::Output: Into<Cow<'a, str>>,
//     C: NearMpcCurve,
// {
//     fn tweak(
//         NearMpcDerivationPath {
//             predecessor_id,
//             path,
//         }: NearMpcDerivationPath<'a, P>,
//     ) -> C::Tweak {
//         // derive the inner path and convert to string
//         let path = S::derive(path).into();

//         // derive the tweak regardless of the curve
//         // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/lib.rs#L424>
//         let tweak = derive_tweak(&predecessor_id, &path);

//         // convert to curve-specific tweak
//         C::tweak(tweak)
//     }
// }

// pub trait NearMpcCurve: DerivableCurve + sealed::Sealed {
//     fn tweak(tweak: Tweak) -> Self::Tweak;
// }

// mod sealed {
//     pub trait Sealed {}
// }
