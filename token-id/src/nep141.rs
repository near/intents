use std::{fmt, str::FromStr};

use near_sdk::{
    AccountId, near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};

use crate::{TokenIdType, error::TokenIdError};

#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[serde_with(crate = "::near_sdk::serde_with")]
pub struct Nep141TokenId {
    pub contract_id: AccountId,
}

impl Nep141TokenId {
    pub fn new(contract_id: impl Into<AccountId>) -> Self {
        Self {
            contract_id: contract_id.into(),
        }
    }
}

impl std::fmt::Debug for Nep141TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.contract_id)
    }
}

impl std::fmt::Display for Nep141TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for Nep141TokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            contract_id: data.parse()?,
        })
    }
}

impl From<&Nep141TokenId> for TokenIdType {
    #[inline]
    fn from(_: &Nep141TokenId) -> Self {
        Self::Nep141
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: Nep141TokenId) {
        let s = token_id.to_string();
        let got: Nep141TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }
}
