#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

use defuse_outlayer_crypto::Curve;

pub trait DerivableCurve: Curve {
    type Tweak;

    #[must_use]
    fn derive_public_key(master_pk: &Self::PublicKey, tweak: &Self::Tweak) -> Self::PublicKey;
}

// #[autoimpl(for<T: trait + ?Sized>
//     &T,
//     &mut T,
//     Box<T>,
//     Rc<T>,
//     Arc<T>
// )]
// #[impl_tools::autoimpl(
//     for<T: trait + ToOwned + ?Sized>
//     Cow<'_, T>
// )]
// pub trait AdditiveDerivationScheme<C, P>
// where
//     C: Curve + ?Sized,
//     P: ?Sized,
// {
//     fn derive_public_key(master_pk: &C::PublicKey, path: &P) -> C::PublicKey;
// }

// pub struct Additive;

// #[cfg(feature = "signing")]
// #[autoimpl(for<T: trait + ?Sized>
//     &T,
//     &mut T,
//     Box<T>,
//     Rc<T>,
//     Arc<T>
// )]
// #[impl_tools::autoimpl(
//     for<T: trait + ToOwned + ?Sized>
//     Cow<'_, T>
// )]
// pub trait DeriveSigner<C, S, P>
// where
//     C: Curve + ?Sized,
//     S: AdditiveDerivationScheme<C, P> + ?Sized,
//     P: ?Sized,
// {
//     fn public_key(&self) -> C::PublicKey;

//     /// Sign given message with a secret key **internally** derived for given
//     /// [`path`](DerivableCurve::Path).
//     ///
//     /// NOTE: the returned signatures might be non-deterministic, i.e.
//     /// implementations MAY return different signatures for the same
//     /// `path` and `msg`.
//     fn derive_sign(&self, path: &P, msg: &C::Message) -> C::Signature;

//     fn derive_public_key(&self, path: &P) -> C::PublicKey {
//         let master_pk = self.public_key();
//         S::derive_public_key(&master_pk, path)
//     }
// }

// #[autoimpl(for<T: trait + ?Sized>
//     &T,
//     &mut T,
//     Box<T>,
//     Rc<T>,
//     Arc<T>
// )]
// #[impl_tools::autoimpl(
//     for<T: trait + ToOwned + ?Sized>
//     Cow<'_, T>
// )]
// pub trait DerivationScheme<C: Curve + ?Sized, P: ?Sized> {
//     type State: ?Sized;

//     fn derive_public_key(state: &Self::State, path: &P) -> C::PublicKey;
// }

// #[cfg(feature = "signing")]
// #[autoimpl(for<T: trait + ?Sized>
//     &T,
//     &mut T,
//     Box<T>,
//     Rc<T>,
//     Arc<T>
// )]
// #[impl_tools::autoimpl(
//     for<T: trait + ToOwned + ?Sized>
//     Cow<'_, T>
// )]
// /// Signer for [`DerivableCurve`]
// pub trait DeriveSigner<C: Curve + ?Sized, P: ?Sized>: DerivationScheme<C, P>
// {
//     /// Sign given message with a secret key **internally** derived for given
//     /// [`path`](DerivableCurve::Path).
//     ///
//     /// NOTE: the returned signatures might be non-deterministic, i.e.
//     /// implementations MAY return different signatures for the same
//     /// `path` and `msg`.
//     fn derive_sign(&self, path: &P, msg: &C::Message) -> C::Signature;

//     // /// Helper method to derive public key from [root](Self::public_key)
//     // /// for given [path](DerivableCurve::Path)
//     // fn derive_public_key(&self, path: &P) -> C::PublicKey {
//     //     S::derive_public_key(&master_pk, path)
//     // }
// }

// / A **non-hardened** public key derivation scheme, i.e. when child public
// / key can be derived from the root one without knowing any secret.
// /
// / `Path` is used to derive both [public](DerivationScheme::derive_public_key)
// / and [signing](DeriveSigner::derive_sign) keys for.
// /
// / The derivation is **non-hierarchical** (or "plain"): derived
// / keys **do not** form a tree-like structure. Instead, child keys
// / are all derived from a single root key and can be considered as
// / "peers" to each other.
// pub trait PublicKeyDerivationScheme<C: Curve + ?Sized, P: ?Sized> {
//     /// A path to derive both [public](DerivationScheme::derive_public_key) and
//     /// [signing](DeriveSigner::derive_sign) keys for.
//     /// Typically, it would be an output of a cryptographic hash function.
//     // type Path: ?Sized;
//     // type Curve: Curve;

//     /// Derive public key from master with given `path` being an output o.
//     fn derive_public_key(master_pk: &C::PublicKey, path: &P) -> C::PublicKey;
// }

// pub struct AdditiveDerivation;

// /// Derivable public key.
// /// See [`DerivableCurve`].
// pub trait DerivablePublicKey<S>
// where
//     S: DerivationScheme + ?Sized,
//     S::Curve: Curve<PublicKey = Self>,
// {
//     /// Derive public key with given [path](DerivableCurve::Path).
//     #[must_use]
//     fn derive_from_tweak(&self, tweak: &S::Tweak) -> Self;

//     // TODO: docs
//     #[must_use]
//     fn derive(&self, path: &str) -> Self {
//         self.derive_from_tweak(&S::tweak(path))
//     }
// }
