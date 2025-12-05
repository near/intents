pub use defuse_nep245::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[cfg(any(feature = "arbitrary", test))]
use arbitrary_with::{Arbitrary, As};
#[cfg(any(feature = "arbitrary", test))]
use defuse_near_utils::arbitrary::ArbitraryAccountId;

use crate::{TokenIdType, error::TokenIdError};

#[cfg_attr(any(feature = "arbitrary", test), derive(Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Nep245TokenId {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
    )]
    pub contract_id: AccountId,

    pub mt_token_id: TokenId,
}

impl Nep245TokenId {
    pub const fn new(contract_id: AccountId, mt_token_id: TokenId) -> Self {
        Self {
            contract_id,
            mt_token_id,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn contract_id(&self) -> &AccountIdRef {
        &self.contract_id
    }

    pub const fn mt_token_id(&self) -> &TokenId {
        &self.mt_token_id
    }

    pub fn into_contract_id_and_mt_token_id(self) -> (AccountId, TokenId) {
        (self.contract_id, self.mt_token_id)
    }
}

impl std::fmt::Debug for Nep245TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.contract_id(), self.mt_token_id())
    }
}

impl std::fmt::Display for Nep245TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for Nep245TokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (contract_id, token_id) = data
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(Self::new(contract_id.parse()?, token_id.to_string()))
    }
}

impl From<&Nep245TokenId> for TokenIdType {
    #[inline]
    fn from(_: &Nep245TokenId) -> Self {
        Self::Nep245
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: Nep245TokenId) {
        let s = token_id.to_string();
        let got: Nep245TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }
}
