use std::{borrow::Cow, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

use crate::{DerivableCurve, DerivationSchema};

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DeriveSigner<C, P>: DerivationSchema<C, P, Output = C::Tweak>
where
    C: DerivableCurve + ?Sized,
{
    fn public_key(&self) -> C::PublicKey;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();
        let tweak = self.derive_path(path);

        C::derive_public_key(&master_pk, &tweak)
    }
}

// pub trait DynDeriveSigner<C, P>
// where
//     C: DerivableCurve + ?Sized,
// {
//     fn schema_dyn<'a>(&'a self) -> Box<dyn DerivationSchema<C, P, Output = C::Tweak> + 'a>
//     where
//         C: 'a,
//         P: 'a;

//     fn public_key_dyn(&self) -> C::PublicKey;

//     fn derive_sign_dyn(&self, path: P, msg: &C::Message) -> C::Signature;

//     fn derive_public_key_dyn(&self, path: P) -> C::PublicKey {
//         let master_pk = self.public_key_dyn();
//         let tweak = self.schema_dyn().derive(path);

//         C::derive_public_key(&master_pk, &tweak)
//     }
// }

// impl<C, P, S> DynDeriveSigner<C, P> for S
// where
//     C: DerivableCurve + ?Sized,
//     S: DeriveSigner<C, P>,
// {
//     fn schema_dyn<'a>(&'a self) -> Box<dyn DerivationSchema<C, P, Output = C::Tweak> + 'a>
//     where
//         C: 'a,
//         P: 'a,
//     {
//         Box::new(self.schema())
//     }

//     fn public_key_dyn(&self) -> C::PublicKey {
//         self.public_key()
//     }

//     fn derive_sign_dyn(&self, path: P, msg: &C::Message) -> C::Signature {
//         self.derive_sign(path, msg)
//     }
// }

#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::Debug;

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

    #[track_caller]
    pub fn assert_roundtrip_expected<S, C, P>(
        root_sk: &S,
        path: P,
        msg: &C::Message,
        expected_derived_pk: &C::PublicKey,
    ) -> C::Signature
    where
        S: DeriveSigner<C, P>,
        C: DerivableCurve,
        C::PublicKey: PartialEq + Debug,
        P: Clone,
    {
        let (derived_pk, signature) = assert_roundtrip(root_sk, path, msg);
        assert_eq!(
            &derived_pk, expected_derived_pk,
            "derived public key has changed"
        );
        signature
    }
}
