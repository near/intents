mod ops;

pub use self::ops::*;

pub use defuse_near_promise as promise;

use defuse_near_promise::NearPromise;

use near_sdk::{Gas, NearToken, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Operations to apply
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<WalletOp>,

    /// Promises to execute (fan-out).
    ///
    /// NOTE: all created promises are executed concurrently and independently
    /// of each other, without waiting on previous ones to complete.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub out: Vec<NearPromise>,
}

impl Request {
    #[inline]
    pub const fn new() -> Self {
        Self {
            ops: Vec::new(),
            out: Vec::new(),
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
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
    pub fn out(mut self, out: impl IntoIterator<Item = NearPromise>) -> Self {
        self.out.extend(out);
        self
    }

    /// Returns total NEAR deposit for all promises in this request
    pub fn total_deposit(&self) -> NearToken {
        self.out
            .iter()
            .map(NearPromise::total_deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    /// Returns an esitmate of mininum gas required to execute all
    /// promises in this request
    pub fn estimate_gas(&self) -> Gas {
        self.out
            .iter()
            .map(NearPromise::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
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

impl Extend<NearPromise> for Request {
    fn extend<T: IntoIterator<Item = NearPromise>>(&mut self, iter: T) {
        self.out.extend(iter);
    }
}

impl FromIterator<NearPromise> for Request {
    fn from_iter<T: IntoIterator<Item = NearPromise>>(iter: T) -> Self {
        let mut r = Self::new();
        r.extend(iter);
        r
    }
}
