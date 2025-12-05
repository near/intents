pub use near_contract_standards::non_fungible_token::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{AccountId, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[cfg(any(feature = "arbitrary", test))]
use arbitrary_with::{Arbitrary, As};
#[cfg(any(feature = "arbitrary", test))]
use defuse_near_utils::arbitrary::ArbitraryAccountId;

use crate::{TokenIdError, TokenIdType};

#[cfg_attr(any(feature = "arbitrary", test), derive(Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Nep171TokenId {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
    )]
    pub contract_id: AccountId,

    pub nft_token_id: TokenId,
}

impl Nep171TokenId {
    pub const fn new(contract_id: AccountId, nft_token_id: TokenId) -> Self {
        Self {
            contract_id,
            nft_token_id,
        }
    }
}

impl std::fmt::Debug for Nep171TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.contract_id, &self.nft_token_id)
    }
}

impl std::fmt::Display for Nep171TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for Nep171TokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (contract_id, token_id) = data
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(Self::new(contract_id.parse()?, token_id.to_string()))
    }
}

impl From<&Nep171TokenId> for TokenIdType {
    #[inline]
    fn from(_: &Nep171TokenId) -> Self {
        Self::Nep171
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: Nep171TokenId) {
        let s = token_id.to_string();
        let got: Nep171TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }
}
