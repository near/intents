use std::fmt::Display;

use near_sdk::{FunctionError, env};

pub trait PanicError {
    #[inline]
    #[track_caller]
    fn panic_str(&self) -> !
    where
        Self: AsRef<str>,
    {
        env::panic_str(self.as_ref())
    }

    #[inline]
    #[track_caller]
    fn panic_static_str(self) -> !
    where
        Self: Into<&'static str>,
    {
        self.into().panic_str()
    }

    #[inline]
    #[track_caller]
    fn panic_display(&self) -> !
    where
        Self: Display,
    {
        self.to_string().panic_str()
    }
}
impl<E> PanicError for E {}

pub trait UnwrapOrPanic<T> {
    fn unwrap_or_panic(self) -> T;
}

impl<T> UnwrapOrPanic<T> for Option<T> {
    #[inline]
    #[track_caller]
    fn unwrap_or_panic(self) -> T {
        self.unwrap_or_else(|| env::abort())
    }
}

impl<T, E> UnwrapOrPanic<T> for Result<T, E>
where
    E: FunctionError,
{
    #[inline]
    #[track_caller]
    fn unwrap_or_panic(self) -> T
    where
        E: FunctionError,
    {
        self.unwrap_or_else(|err| err.panic())
    }
}

pub trait UnwrapOrPanicError<T, E> {
    fn unwrap_or_panic_str(self) -> T
    where
        E: AsRef<str>;

    fn unwrap_or_panic_static_str(self) -> T
    where
        E: Into<&'static str>;

    fn unwrap_or_panic_display(self) -> T
    where
        E: Display;
}

impl<T, E> UnwrapOrPanicError<T, E> for Result<T, E> {
    #[inline]
    #[track_caller]
    fn unwrap_or_panic_str(self) -> T
    where
        E: AsRef<str>,
    {
        self.unwrap_or_else(|err| err.panic_str())
    }

    #[inline]
    #[track_caller]
    fn unwrap_or_panic_static_str(self) -> T
    where
        E: Into<&'static str>,
    {
        self.unwrap_or_else(|err| err.panic_static_str())
    }

    #[inline]
    #[track_caller]
    fn unwrap_or_panic_display(self) -> T
    where
        E: Display,
    {
        self.unwrap_or_else(|err| err.panic_display())
    }
}
