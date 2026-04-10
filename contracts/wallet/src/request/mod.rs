mod ops;
mod promise;

pub use self::{ops::*, promise::*};

use near_sdk::near;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<WalletOp>,

    #[serde(default, skip_serializing_if = "PromiseDAG::is_empty")]
    pub out: PromiseDAG,
}

impl Request {
    #[inline]
    pub const fn new() -> Self {
        Self {
            ops: Vec::new(),
            out: PromiseDAG::new(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty() && self.out.is_empty()
    }

    #[must_use]
    #[inline]
    pub fn ops(mut self, ops: impl IntoIterator<Item = WalletOp>) -> Self {
        self.extend(ops);
        self
    }

    #[must_use]
    #[inline]
    pub fn out(mut self, out: impl Into<PromiseDAG>) -> Self {
        self.out = out.into();
        self
    }
}

impl Extend<WalletOp> for Request {
    fn extend<T: IntoIterator<Item = WalletOp>>(&mut self, iter: T) {
        self.ops.extend(iter);
    }
}

impl FromIterator<WalletOp> for Request {
    fn from_iter<T: IntoIterator<Item = WalletOp>>(iter: T) -> Self {
        let mut r = Self::new();
        r.extend(iter);
        r
    }
}

impl Extend<PromiseSingle> for Request {
    fn extend<T: IntoIterator<Item = PromiseSingle>>(&mut self, iter: T) {
        self.out.extend(iter);
    }
}

impl FromIterator<PromiseSingle> for Request {
    fn from_iter<T: IntoIterator<Item = PromiseSingle>>(iter: T) -> Self {
        let mut r = Self::new();
        r.extend(iter);
        r
    }
}
