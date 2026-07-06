#[cfg(feature = "hex")]
pub mod hex;
// TODO: more adaptors

use std::{rc::Rc, sync::Arc};

pub use anyhow::{Error, Result};

use impl_tools::autoimpl;

#[autoimpl(for<S: trait + ?Sized> &S, &mut S, Box<S>, Rc<S>, Arc<S>)]
pub trait Schema<T> {
    type Output;

    fn derive(&self, input: T) -> Result<Self::Output>;
}

pub trait SchemaExt<T>: Schema<T> {
    #[inline]
    fn and_then<O>(self, outer: O) -> AndThen<Self, O>
    where
        Self: Sized,
        O: Schema<Self::Output>,
    {
        AndThen { inner: self, outer }
    }

    #[inline]
    fn by_ref(&self) -> &Self {
        self
    }
}
impl<T, S> SchemaExt<T> for S where S: Schema<T> {}

#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl<T> Schema<T> for Identity {
    type Output = T;

    #[inline]
    fn derive(&self, input: T) -> Result<Self::Output> {
        Ok(input)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaFn<F>(F);

impl<T, F, O> Schema<T> for SchemaFn<F>
where
    F: Fn(T) -> Result<O>,
{
    type Output = O;

    #[inline]
    fn derive(&self, input: T) -> Result<Self::Output> {
        (self.0)(input)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Value<T>(T);

impl<T> Value<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Schema<()> for Value<T>
where
    T: Clone,
{
    type Output = T;

    #[inline]
    fn derive(&self, _input: ()) -> Result<Self::Output> {
        Ok(self.0.clone())
    }
}

impl<T> AsRef<T> for Value<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AndThen<I, O> {
    inner: I,
    outer: O,
}

impl<T, I, O> Schema<T> for AndThen<I, O>
where
    I: Schema<T>,
    O: Schema<I::Output>,
{
    type Output = O::Output;

    #[inline]
    fn derive(&self, input: T) -> Result<Self::Output> {
        self.outer.derive(self.inner.derive(input)?)
    }
}

pub type BoxSchema<'a, T, O> = Box<dyn Schema<T, Output = O> + 'a>;
