mod ops;

pub use self::ops::*;

pub use defuse_near_promise::*;

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Internal operations to apply
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub internal: Vec<WalletOp>,

    /// External promises to execute (fan-out).
    ///
    /// NOTE: all created promises are executed concurrently and independently
    /// of each other, without waiting on previous ones to complete.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub external: Vec<NearPromise>,
}

impl Request {
    #[inline]
    pub const fn new() -> Self {
        Self {
            internal: Vec::new(),
            external: Vec::new(),
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.internal.is_empty() && self.external.is_empty()
    }

    #[must_use]
    #[inline]
    pub fn internal(mut self, ops: impl IntoIterator<Item = WalletOp>) -> Self {
        self.internal.extend(ops);
        self
    }

    #[must_use]
    #[inline]
    pub fn external(mut self, promises: impl IntoIterator<Item = NearPromise>) -> Self {
        self.external.extend(promises);
        self
    }

    /// Returns total NEAR deposit for all promises in this request
    pub fn total_deposit(&self) -> NearToken {
        self.external
            .iter()
            .map(NearPromise::total_deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    /// Returns an esitmate of mininum gas required to execute all
    /// promises in this request
    pub fn estimate_gas(&self) -> Gas {
        self.external
            .iter()
            .map(NearPromise::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
    }
}

impl Extend<WalletOp> for Request {
    fn extend<T: IntoIterator<Item = WalletOp>>(&mut self, iter: T) {
        self.internal.extend(iter);
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
        self.external.extend(iter);
    }
}

impl FromIterator<NearPromise> for Request {
    fn from_iter<T: IntoIterator<Item = NearPromise>>(iter: T) -> Self {
        let mut r = Self::new();
        r.extend(iter);
        r
    }
}
