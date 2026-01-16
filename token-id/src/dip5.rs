pub use defuse_nep245::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{AccountId, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::{TokenIdType, error::TokenIdError};

#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Dip5TokenId {
    pub minter_id: AccountId,

    pub token_id: TokenId,
}

impl Dip5TokenId {
    pub fn new(minter_id: impl Into<AccountId>, token_id: impl Into<TokenId>) -> Self {
        Self {
            minter_id: minter_id.into(),
            token_id: token_id.into(),
        }
    }
}

impl std::fmt::Debug for Dip5TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.minter_id, &self.token_id)
    }
}

impl std::fmt::Display for Dip5TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for Dip5TokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (contract_id, token_id) = data
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(Self::new(contract_id.parse::<AccountId>()?, token_id))
    }
}

impl From<&Dip5TokenId> for TokenIdType {
    #[inline]
    fn from(_: &Dip5TokenId) -> Self {
        Self::Dip5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: Dip5TokenId) {
        let s = token_id.to_string();
        let got: Dip5TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }
}
