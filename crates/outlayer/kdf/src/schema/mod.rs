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
    C: DerivableCurve + ?Sized,
{
    type Output;

    fn derive_path(&self, path: P) -> Self::Output;
}

#[derive(Default)]
pub struct Identity;

impl<C, T> DerivationSchema<C, T> for Identity
where
    C: DerivableCurve + ?Sized,
{
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

pub struct SchemaFn<C: ?Sized, F> {
    f: F,
    _curve: PhantomData<C>,
}

impl<C: ?Sized, F> SchemaFn<C, F> {
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
    C: DerivableCurve + ?Sized,
    F: Fn(P) -> O,
{
    type Output = O;

    fn derive_path(&self, path: P) -> Self::Output {
        (self.f)(path)
    }
}
