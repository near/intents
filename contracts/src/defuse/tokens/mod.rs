pub mod nep141;
pub mod nep171;
pub mod nep245;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use std::collections::{btree_map, BTreeMap};

use impl_tools::autoimpl;
use near_account_id::ParseAccountError;
use near_sdk::{near, AccountId};
use serde_with::{serde_as, DeserializeFromStr, DisplayFromStr, SerializeDisplay};
use strum::{EnumDiscriminants, EnumString};
use thiserror::Error as ThisError;

use crate::utils::{
    cleanup::DefaultMap,
    integer::{CheckedAdd, CheckedSub},
};

use super::{DefuseError, Result};

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumDiscriminants,
    SerializeDisplay,
    DeserializeFromStr,
)]
#[strum_discriminants(
    name(TokenIdType),
    derive(strum::Display, EnumString),
    strum(serialize_all = "snake_case")
)]
#[near(serializers = [borsh])]
pub enum TokenId {
    Nep141(
        /// Contract
        AccountId,
    ),
    Nep171(
        /// Contract
        AccountId,
        /// Token ID
        near_contract_standards::non_fungible_token::TokenId,
    ),
    Nep245(
        /// Contract
        AccountId,
        /// Token ID
        crate::nep245::TokenId,
    ),
}

impl Debug for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nep141(contract_id) => {
                write!(f, "{}:{}", TokenIdType::Nep141, contract_id)
            }
            Self::Nep171(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep171, contract_id, token_id)
            }
            Self::Nep245(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep245, contract_id, token_id)
            }
        }
    }
}

impl Display for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for TokenId {
    type Err = ParseTokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(match typ.parse()? {
            TokenIdType::Nep141 => Self::Nep141(data.parse()?),
            TokenIdType::Nep171 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep171(contract_id.parse()?, token_id.to_string())
            }
            TokenIdType::Nep245 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep245(contract_id.parse()?, token_id.to_string())
            }
        })
    }
}

#[derive(Debug, ThisError)]
pub enum ParseTokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};

    impl JsonSchema for TokenId {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn json_schema(gen: &mut SchemaGenerator) -> Schema {
            String::json_schema(gen)
        }

        fn is_referenceable() -> bool {
            false
        }
    }
}

#[derive(Debug, Clone)]
#[autoimpl(Default)]
#[autoimpl(Deref using self.0)]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[serde(bound(serialize = "T: Display", deserialize = "T: FromStr<Err: Display>"))] // HACK
pub struct TokenAmounts<T>(
    /// [`BTreeMap`] ensures deterministic order
    #[serde_as(as = "BTreeMap<_, DisplayFromStr>")]
    BTreeMap<TokenId, T>,
);

impl<A> TokenAmounts<A> {
    #[inline]
    fn try_apply<E>(&mut self, token_id: TokenId, f: impl FnOnce(A) -> Result<A, E>) -> Result<A, E>
    where
        A: Default + Eq + Copy,
    {
        let mut d = self.0.entry_or_default(token_id);
        *d = f(*d)?;
        Ok(*d)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<A> TokenAmounts<A> {
    #[inline]
    pub fn add<T>(&mut self, token_id: TokenId, amount: T) -> Result<A>
    where
        A: CheckedAdd<T> + Default + Eq + Copy,
    {
        self.try_apply(token_id, |a| {
            a.checked_add(amount).ok_or(DefuseError::IntegerOverflow)
        })
    }

    #[inline]
    pub fn sub<T>(&mut self, token_id: TokenId, amount: T) -> Result<A>
    where
        A: CheckedSub<T> + Default + Eq + Copy,
    {
        self.try_apply(token_id, |a| {
            a.checked_sub(amount).ok_or(DefuseError::IntegerOverflow)
        })
    }

    #[inline]
    pub fn with_add<T>(mut self, amounts: impl IntoIterator<Item = (TokenId, T)>) -> Result<Self>
    where
        A: CheckedAdd<T> + Default + Eq + Copy,
    {
        for (token_id, amount) in amounts {
            self.add(token_id, amount)?;
        }
        Ok(self)
    }
}

impl<T> IntoIterator for TokenAmounts<T> {
    type Item = (TokenId, T);

    type IntoIter = btree_map::IntoIter<TokenId, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a TokenAmounts<T> {
    type Item = (&'a TokenId, &'a T);

    type IntoIter = btree_map::Iter<'a, TokenId, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn invariant() {
        let [t1, t2] = ["t1.near", "t2.near"].map(|t| TokenId::Nep141(t.parse().unwrap()));

        assert!(TokenAmounts::<()>::default().is_empty());
        assert!(TokenAmounts::<i32>::default()
            .with_add([(t1.clone(), 0)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<i32>::default()
            .with_add([(t1.clone(), 1)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<i32>::default()
            .with_add([(t1.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(TokenAmounts::<i32>::default()
            .with_add([(t1.clone(), 1), (t1.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<i32>::default()
            .with_add([(t1.clone(), 1), (t1.clone(), -1), (t2.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(TokenAmounts::<i32>::default()
            .with_add([
                (t1.clone(), 1),
                (t1.clone(), -1),
                (t2.clone(), -1),
                (t2.clone(), 1)
            ])
            .unwrap()
            .is_empty());
    }
}
