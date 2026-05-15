use std::{borrow::Cow, marker::PhantomData, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

use crate::DerivableCurve;

#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "hex")]
pub mod hex;

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DerivationSchema<C, P>
where
    C: DerivableCurve,
{
    type Output;

    fn derive_path(&self, path: P) -> Self::Output;

    fn derive_public_key_from_master(&self, master_pk: &C::PublicKey, path: P) -> C::PublicKey
    where
        C: DerivableCurve<Tweak = Self::Output>,
    {
        let tweak = self.derive_path(path);
        C::derive_public_key(master_pk, &tweak)
    }
}

// TODO: type params
pub trait SchemaExt {
    #[inline]
    fn then<S>(self, then: S) -> Then<Self, S>
    where
        Self: Sized,
    {
        Then(self, then)
    }
}

impl<S> SchemaExt for S {}

#[derive(Default)]
pub struct Identity;

impl<C, T> DerivationSchema<C, T> for Identity
where
    C: DerivableCurve,
{
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

// TODO: implement DerivableSigner, too?
#[derive(Default)]
pub struct Then<A, B>(A, B);

impl<A, B> Then<A, B> {
    pub const fn new(first: A, second: B) -> Self {
        Self(first, second)
    }
}

impl<C, P, A, B> DerivationSchema<C, P> for Then<A, B>
where
    C: DerivableCurve,
    A: DerivationSchema<C, P>,
    B: DerivationSchema<C, A::Output>,
{
    type Output = B::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.1.derive_path(self.0.derive_path(path))
    }
}

pub struct SchemaFn<C, F> {
    f: F,
    _curve: PhantomData<C>,
}

impl<C, F> SchemaFn<C, F> {
    #[inline]
    pub const fn new(f: F) -> Self {
        Self {
            f,
            _curve: PhantomData,
        }
    }
}

impl<C, P, F, O> DerivationSchema<C, P> for SchemaFn<C, F>
where
    C: DerivableCurve,
    F: Fn(P) -> O,
{
    type Output = O;

    fn derive_path(&self, path: P) -> Self::Output {
        (self.f)(path)
    }
}
