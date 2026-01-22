pub use defuse_nep245::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{AccountId, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::{TokenIdType, error::TokenIdError};

// Intent mintable token - can be minted only by intents 'ImtMint'
#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct ImtTokenId {
    pub minter_id: AccountId,

    pub token_id: TokenId,
}

impl ImtTokenId {
    pub fn new(minter_id: impl Into<AccountId>, token_id: impl Into<TokenId>) -> Self {
        Self {
            minter_id: minter_id.into(),
            token_id: token_id.into(),
        }
    }
}

impl std::fmt::Debug for ImtTokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.minter_id, &self.token_id)
    }
}

impl std::fmt::Display for ImtTokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for ImtTokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (minter_id, token_id) = data
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(Self::new(minter_id.parse::<AccountId>()?, token_id))
    }
}

impl From<&ImtTokenId> for TokenIdType {
    #[inline]
    fn from(_: &ImtTokenId) -> Self {
        Self::Imt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: ImtTokenId) {
        let s = token_id.to_string();
        let got: ImtTokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }
}
