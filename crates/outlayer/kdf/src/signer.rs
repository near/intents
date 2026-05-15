use std::{borrow::Cow, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

use crate::{DerivableCurve, DerivationSchema};

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DeriveSigner<C, P>
where
    C: DerivableCurve + ?Sized,
{
    type Schema<'a>: DerivationSchema<C, P, Output = C::Tweak>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_>;

    fn public_key(&self) -> C::PublicKey;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();
        let tweak = self.schema().derive(path);

        C::derive_public_key(&master_pk, &tweak)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::Debug;

    use super::*;

    #[track_caller]
    pub fn assert_roundtrip<S, C, P>(
        root_sk: &S,
        tweak: P,
        msg: &C::Message,
    ) -> (C::PublicKey, C::Signature)
    where
        S: DeriveSigner<C, P>,
        C: DerivableCurve,
        P: Clone,
    {
        let derived_pk = root_sk.derive_public_key(tweak.clone());
        let signature = root_sk.derive_sign(tweak, msg);

        assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

        (derived_pk, signature)
    }

    #[track_caller]
    pub fn assert_roundtrip_expected<S, C, P>(
        root_sk: &S,
        tweak: P,
        msg: &C::Message,
        expected_derived_pk: &C::PublicKey,
    ) -> C::Signature
    where
        S: DeriveSigner<C, P>,
        C: DerivableCurve,
        C::PublicKey: PartialEq + Debug,
        P: Clone,
    {
        let (derived_pk, signature) = assert_roundtrip(root_sk, tweak, msg);
        assert_eq!(
            &derived_pk, expected_derived_pk,
            "derived public key has changed"
        );
        signature
    }
}
