use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::token_id::error::TokenIdError;

#[cfg(any(feature = "arbitrary", test))]
use arbitrary_with::{Arbitrary, As};
#[cfg(any(feature = "arbitrary", test))]
use defuse_near_utils::arbitrary::ArbitraryAccountId;

#[cfg_attr(any(feature = "arbitrary", test), derive(Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Nep141TokenId {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
    )]
    contract_id: AccountId,
}

impl Nep141TokenId {
    pub const fn new(contract_id: AccountId) -> Self {
        Self { contract_id }
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        self.contract_id.as_ref()
    }

    pub fn into_contract_id(self) -> AccountId {
        self.contract_id
    }
}

impl std::fmt::Debug for Nep141TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.contract_id())
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
