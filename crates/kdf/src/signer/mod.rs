#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

use std::{borrow::Cow, rc::Rc, sync::Arc};

use defuse_kdf_crypto::Curve;
use impl_tools::autoimpl;

use crate::{BoxSchema, Derive, DeriveExt, Schema};

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
/// A signer that can sign messages by **internally** deriving signing keys
/// according to its public key derivation [schema](DeriveSigner::schema).
pub trait DeriveSigner<C: Curve, P> {
    /// [`Schema`] for public key derivation.
    /// See [`.schema()`](DeriveSigner::schema) for details.
    type Schema<'a>: Schema<P, Output = C::PublicKey>
    where
        Self: 'a;

    /// Construct [schema](Schema) for public key derivation.
    ///
    /// For _non-hardened_ (i.e. "public") derivaton, the returned schema
    /// shoudn't contain any secret information, so that derivation can be
    /// performed by clients fully offline, without any interactions with the
    /// signer, since all parameters are public.
    ///
    /// For _hardened_ derivation, this would typically reference `self`, since
    /// public keys can be only derived by knowing a master signing key.
    ///
    /// See [`.derive_public_key()`](DeriveSigner::derive_public_key)
    /// shorthand.
    fn schema(&self) -> Self::Schema<'_>;

    /// Sign given message with a secret key **internally** derived
    /// for given `path` according to [`schema`](DeriveSigner::schema).
    ///
    /// **NOTE**: the returned signatures MIGHT be non-deterministic, i.e.
    /// implementations MAY return different signatures for the same
    /// `path` and `msg`.
    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    /// Helper method to [derive](Schema::derive_path) public key for given
    /// `path` via [`.schema()`](DeriveSigner::Schema)
    #[inline]
    fn derive_public_key(&self, path: P) -> C::PublicKey {
        self.schema().derive_path(path)
    }
}

impl<C, P, S, D> DeriveSigner<C, P> for Derive<S, D>
where
    C: Curve,
    D: Schema<P>,
    S: DeriveSigner<C, D::Output>,
{
    type Schema<'a>
        = Derive<S::Schema<'a>, &'a D>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        self.0.schema().derive(&self.1)
    }

    #[inline]
    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.0.derive_sign(self.1.derive_path(path), msg)
    }
}

/// Object-safe version of [`DeriveSigner`] trait.
pub trait DynDeriveSigner<C: Curve, P> {
    fn schema_dyn<'a>(&'a self) -> BoxSchema<'a, P, C::PublicKey>
    where
        P: 'a;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;
}

impl<C, P, S> DynDeriveSigner<C, P> for S
where
    C: Curve,
    S: DeriveSigner<C, P>,
{
    #[inline]
    fn schema_dyn<'a>(&'a self) -> BoxSchema<'a, P, C::PublicKey>
    where
        P: 'a,
    {
        Box::new(self.schema())
    }

    #[inline]
    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        DeriveSigner::<C, P>::derive_sign(self, path, msg)
    }
}

impl<C: Curve, P> DeriveSigner<C, P> for dyn DynDeriveSigner<C, P> {
    type Schema<'a>
        = BoxSchema<'a, P, C::PublicKey>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        self.schema_dyn()
    }

    #[inline]
    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        DynDeriveSigner::<C, P>::derive_sign(self, path, msg)
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[track_caller]
    pub fn assert_roundtrip<S, C, P>(
        root_sk: &S,
        path: P,
        msg: &C::Message,
    ) -> (C::PublicKey, C::Signature)
    where
        C: Curve,
        S: DeriveSigner<C, P>,
        P: Clone,
    {
        let derived_pk = root_sk.derive_public_key(path.clone());
        let signature = root_sk.derive_sign(path, msg);

        assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

        (derived_pk, signature)
    }
}
