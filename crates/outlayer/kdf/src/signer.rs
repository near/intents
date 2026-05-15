use std::{borrow::Cow, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

use crate::{DerivableCurve, DerivationSchema};

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DeriveSigner<C, P>: DerivationSchema<C, P, Output = C::Tweak>
where
    C: DerivableCurve,
{
    fn public_key(&self) -> C::PublicKey;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();
        let tweak = self.derive_path(path);

        C::derive_public_key(&master_pk, &tweak)
    }
}

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
