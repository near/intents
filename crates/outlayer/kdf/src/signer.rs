use std::{borrow::Cow, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

use crate::{DerivableCurve, DerivationSchema};

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DeriveSigner<C, P>
where
    C: DerivableCurve,
{
    /// [`DerivationSchema`] implemented by [`.derive_sign()`](DeriveSigner::derive_sign).
    type Schema<'a>: DerivationSchema<P, Output = C::Tweak>
    where
        Self: 'a;

    // TODO
    /// Construct [`Schema`](Self::Schema) for public key derivation.
    fn schema(&self) -> Self::Schema<'_>;

    // TODO
    /// Returns []
    fn public_key(&self) -> C::PublicKey;

    // TODO
    /// Sign given message with a secret key **internally** derived
    /// for given `path` according to [`Self::Schema`](DeriveSigner::Schema).
    ///
    /// NOTE: the returned signatures might be non-deterministic, i.e.
    /// implementations MAY return different signatures for the same
    /// `path` and `msg`.
    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    /// Helper method to derive [tweak](DerivableCurve::Tweak) for given `path`
    /// according to [`Schema`](DeriveSigner::Schema)
    fn derive_tweak(&self, path: P) -> C::Tweak {
        self.schema().derive_path(path)
    }

    /// Helper method to [derive](DerivableCurve::derive_public_key) public
    /// key from [master](DeriveSigner::public_key) for given `path` according
    /// to [`Schema`](DeriveSigner::Schema)
    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();
        let tweak = self.derive_tweak(path);

        C::derive_public_key(&master_pk, &tweak)
    }
}

pub trait DynDeriveSigner<C, P>
where
    C: DerivableCurve,
{
    fn schema_dyn<'a>(&'a self) -> Box<dyn DerivationSchema<P, Output = C::Tweak> + 'a>
    where
        P: 'a;

    fn public_key(&self) -> C::PublicKey;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;
}

impl<C, P, S> DynDeriveSigner<C, P> for S
where
    C: DerivableCurve,
    S: DeriveSigner<C, P>,
{
    fn schema_dyn<'a>(&'a self) -> Box<dyn DerivationSchema<P, Output = C::Tweak> + 'a>
    where
        P: 'a,
    {
        Box::new(self.schema())
    }

    fn public_key(&self) -> C::PublicKey {
        DeriveSigner::<C, P>::public_key(self)
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        DeriveSigner::<C, P>::derive_sign(self, path, msg)
    }
}

impl<'l, C, P> DeriveSigner<C, P> for dyn DynDeriveSigner<C, P> + 'l
where
    C: DerivableCurve,
{
    type Schema<'a>
        = Box<dyn DerivationSchema<P, Output = C::Tweak> + 'a>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        self.schema_dyn()
    }

    fn public_key(&self) -> C::PublicKey {
        DynDeriveSigner::<C, P>::public_key(self)
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        DynDeriveSigner::<C, P>::derive_sign(self, path, msg)
    }
}

// #[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
// #[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
// pub trait DeriveSignerSchema<C, P>: DeriveSigner<C, P>
// where
//     C: DerivableCurve,
// {
//     type Schema<'a>: DerivationSchema<P, Output = C::Tweak>
//     where
//         Self: 'a;

//     fn derivation_schema(&self) -> Self::Schema<'_>;
// }

#[cfg(test)]
pub(crate) mod tests {

    use super::*;

    #[track_caller]
    pub fn assert_roundtrip<S, C, P>(
        root_sk: &S,
        path: P,
        msg: &C::Message,
    ) -> (C::PublicKey, C::Signature)
    where
        S: DeriveSigner<C, P>,
        C: DerivableCurve,
        P: Clone,
    {
        let derived_pk = root_sk.derive_public_key(path.clone());
        let signature = root_sk.derive_sign(path, msg);

        assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

        (derived_pk, signature)
    }
}
