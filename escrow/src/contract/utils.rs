use core::convert::Infallible;

use near_sdk::Promise;

// TODO: rename
pub fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}

pub trait ResultExt {
    type Ok;

    fn into_ok(self) -> Self::Ok;
}

impl<T> ResultExt for Result<T, Infallible> {
    type Ok = T;

    fn into_ok(self) -> T {
        match self {
            Ok(v) => v,
        }
    }
}

pub trait PromiseExt: Sized {
    fn maybe_and(self, p: Option<Promise>) -> Promise;
}

impl PromiseExt for Promise {
    #[inline]
    fn maybe_and(self, p: Option<Promise>) -> Promise {
        if let Some(p) = p { self.and(p) } else { self }
    }
}

pub trait MaybePromise {
    fn and_or(self, p: Promise) -> Promise;
    fn then_or(self, p: Promise) -> Promise;
}

impl MaybePromise for Option<Promise> {
    #[inline]
    fn and_or(self, p: Promise) -> Promise {
        if let Some(s) = self { s.and(p) } else { p }
    }

    #[inline]
    fn then_or(self, p: Promise) -> Promise {
        if let Some(s) = self { s.then(p) } else { p }
    }
}

// #[derive(Default)]
// pub struct MaybePromise(Option<Promise>);

// impl MaybePromise {
//     pub const fn new(p: Option<Promise>) -> Self {
//         Self(p)
//     }

//     pub const fn promise(p: Promise) -> Self {
//         Self::new(Some(p))
//     }

//     pub fn and_or(self, and: Promise) -> Promise {
//         self.0.into_iter().fold(and, |b, a| a.and(b))
//     }

//     pub fn then_or(self, then: Promise) -> Promise {
//         self.0.into_iter().fold(then, |b, a| a.then(b))
//     }

//     pub fn into_option(self) -> Option<Promise> {
//         self.0
//     }
// }

// #[derive(Default)]
// pub struct MaybePromiseAnd(MaybePromise);

// impl MaybePromiseAnd {
//     pub fn into_inner(self) -> MaybePromise {
//         self.0
//     }
// }

// impl FromIterator<Promise> for MaybePromiseAnd {
//     fn from_iter<T: IntoIterator<Item = Promise>>(iter: T) -> Self {
//         Self(MaybePromise::new(iter.into_iter().reduce(|a, b| a.and(b))))
//     }
// }

// impl Extend<Promise> for MaybePromiseAnd {
//     fn extend<T: IntoIterator<Item = Promise>>(&mut self, iter: T) {
//         *self = self.0.0.take().into_iter().chain(iter).collect();
//     }
// }

// impl FromIterator<Promise> for MaybePromise {
//     fn from_iter<T: IntoIterator<Item = Promise>>(iter: T) -> Self {
//         Self(iter.into_iter().reduce(|a, b| a.and(b)))
//     }
// }

// impl Extend<Promise> for MaybePromise {
//     fn extend<T: IntoIterator<Item = Promise>>(&mut self, iter: T) {
//         *self = self.0.take().into_iter().chain(iter).collect();
//     }
// }
