mod ops;

pub use self::ops::*;

pub use defuse_near_promise::*;

/// A request containing internal [operations](WalletOp) to apply and external
/// [promises](NearPromise) to create.
///
/// Used directly by [extensions](crate::contract::Wallet::w_execute_extension)
/// and wrapped in [`RequestMessage`](crate::RequestMessage) for
/// [signed requests](crate::contract::Wallet::w_execute_signed).
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(deny_unknown_fields)
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Request {
    /// (optional) Ordered list of internal operations to apply.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub internal: Vec<WalletOp>,

    /// (optional) External promises to execute (fan-out).
    ///
    /// NOTE: all created promises are executed concurrently in arbitrary order
    /// and independently of each other, without waiting on previous ones to
    /// complete.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::serde_adapters::promises")
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

    /// Estimates execution and scheduling gas for this request.
    ///
    /// This is not an upper bound: signature verification, serialization and
    /// runtime costs depend on the message and the signing variant. Relayers
    /// must leave headroom, particularly for large messages and callback arguments.
    pub fn estimate_gas(&self) -> Gas {
        const BASE: Gas = Gas::from_tgas(5);
        const PER_INTERNAL_OP: Gas = Gas::from_ggas(100);

        let gas = BASE.saturating_add(
            PER_INTERNAL_OP.saturating_mul(self.internal.len().try_into().unwrap_or(u64::MAX)),
        );

        // Promises to a token with required metadata also check it first:
        // the wallet schedules `ft_metadata()` and the `w_on_ft_metadata()`
        // callback, which gets the promise's gas and creates the promise.
        let checked = |p: &NearPromise| {
            self.internal.iter().any(|op| {
                matches!(op, WalletOp::RequireFtMetadata { token_id, .. } if *token_id == p.receiver_id)
            })
        };
        self.external
            .iter()
            .map(|p| {
                let gas = p.estimate_gas();
                if !checked(p) {
                    return gas;
                }
                let call = |name: &str, gas| {
                    NearPromise::new(p.receiver_id.clone())
                        .function_call(actions::FunctionCall::name(name).gas(gas))
                        .estimate_gas()
                };
                call("ft_metadata", FT_METADATA_GAS).saturating_add(call(
                    "w_on_ft_metadata",
                    gas.saturating_add(FT_METADATA_CALLBACK_GAS),
                ))
            })
            .fold(gas, Gas::saturating_add)
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

impl From<WalletOp> for Request {
    #[inline]
    fn from(op: WalletOp) -> Self {
        Self::from_iter([op])
    }
}

impl From<NearPromise> for Request {
    #[inline]
    fn from(p: NearPromise) -> Self {
        Self::from_iter([p])
    }
}

#[cfg(test)]
mod tests {
    use near_account_id::AccountId;

    use super::*;

    /// A promise checked by [`WalletOp::RequireFtMetadata`] also costs the
    /// `ft_metadata()` call and its callback: their gas and their creation.
    #[test]
    fn estimate_gas_includes_ft_metadata_check() {
        let p = NearPromise::new("token.near".parse::<AccountId>().unwrap())
            .function_call(actions::FunctionCall::name("ft_transfer").gas(Gas::from_tgas(10)));
        let request = |token_id: &str| {
            Request::new()
                .internal([WalletOp::require_ft_metadata(
                    token_id.parse::<AccountId>().unwrap(),
                    6,
                    "USDC",
                )])
                .external([p.clone()])
        };
        let call = |name: &str, gas| {
            NearPromise::new("wallet.near".parse::<AccountId>().unwrap())
                .function_call(actions::FunctionCall::name(name).gas(gas))
                .estimate_gas()
        };
        assert_eq!(
            request("token.near").estimate_gas(),
            request("other.near")
                .estimate_gas()
                .saturating_sub(p.estimate_gas())
                .saturating_add(call("ft_metadata", FT_METADATA_GAS))
                .saturating_add(call(
                    "w_on_ft_metadata",
                    p.estimate_gas().saturating_add(FT_METADATA_CALLBACK_GAS),
                )),
        );
    }
}
