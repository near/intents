use std::{borrow::Cow, rc::Rc, sync::Arc};

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
}

pub trait DerivationSchemaExt<C, P>: DerivationSchema<C, P>
where
    C: DerivableCurve,
{
    #[inline]
    fn then<S>(self, then: S) -> Then<Self, S>
    where
        Self: Sized,
    {
        Then(self, then)
    }
}

impl<C, P, S> DerivationSchemaExt<C, P> for S
where
    S: DerivationSchema<C, P>,
    C: DerivableCurve,
{
}

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

pub struct SchemaFn<F>(F);

impl<F> SchemaFn<F> {
    #[inline]
    pub const fn new(f: F) -> Self {
        Self(f)
    }
}

impl<C, P, F, O> DerivationSchema<C, P> for SchemaFn<F>
where
    C: DerivableCurve,
    F: Fn(P) -> O,
{
    type Output = O;

    fn derive_path(&self, path: P) -> Self::Output {
        (self.0)(path)
    }
}
