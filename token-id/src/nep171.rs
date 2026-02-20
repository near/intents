pub use near_contract_standards::non_fungible_token::TokenId;

use std::{fmt, str::FromStr};

use near_sdk::{
    AccountId, near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};

use crate::{TokenIdError, TokenIdType};

#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[serde_with(crate = "::near_sdk::serde_with")]
pub struct Nep171TokenId {
    pub contract_id: AccountId,

    pub nft_token_id: TokenId,
}

impl Nep171TokenId {
    pub fn new(contract_id: impl Into<AccountId>, nft_token_id: impl Into<TokenId>) -> Self {
        Self {
            contract_id: contract_id.into(),
            nft_token_id: nft_token_id.into(),
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
        Ok(Self::new(contract_id.parse::<AccountId>()?, token_id))
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
