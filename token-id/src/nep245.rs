pub use defuse_nep245::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{AccountId, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::{TokenIdType, error::TokenIdError};

#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Nep245TokenId {
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
}

impl std::fmt::Debug for Nep245TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.contract_id, &self.mt_token_id)
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
