use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::token_id::{MAX_ALLOWED_TOKEN_ID_LEN, error::TokenIdError};

#[cfg(any(feature = "arbitrary", test))]
use arbitrary_with::{Arbitrary, As, LimitLen};
#[cfg(any(feature = "arbitrary", test))]
use defuse_near_utils::arbitrary::ArbitraryAccountId;

#[cfg_attr(any(feature = "arbitrary", test), derive(Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
pub struct Nep245TokenId {
    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
    )]
    contract_id: AccountId,

    #[cfg_attr(
        any(feature = "arbitrary", test),
        arbitrary(with = As::<LimitLen<MAX_ALLOWED_TOKEN_ID_LEN>>::arbitrary),
    )]
    mt_token_id: defuse_nep245::TokenId,
}

impl Nep245TokenId {
    pub fn new(
        contract_id: AccountId,
        mt_token_id: defuse_nep245::TokenId,
    ) -> Result<Self, TokenIdError> {
        if mt_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(mt_token_id.len()));
        }

        Ok(Self {
            contract_id,
            mt_token_id,
        })
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        &self.contract_id
    }

    pub const fn mt_token_id(&self) -> &defuse_nep245::TokenId {
        &self.mt_token_id
    }

    pub fn into_contract_id_and_mt_token_id(self) -> (AccountId, defuse_nep245::TokenId) {
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
        Self::new(contract_id.parse()?, token_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Unstructured;
    use arbitrary_with::UnstructuredExt;
    use defuse_test_utils::random::{make_arbitrary, random_bytes};
    use rstest::rstest;

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: Nep245TokenId) {
        let s = token_id.to_string();
        let got: Nep245TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }

    #[rstest]
    fn token_id_length(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        let contract_id = u.arbitrary_as::<_, ArbitraryAccountId>().unwrap();
        let token_id: String = u.arbitrary().unwrap();

        let r = Nep245TokenId::new(contract_id, token_id.clone());
        if token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            assert!(matches!(r.unwrap_err(), TokenIdError::TokenIdTooLarge(_)));
        } else {
            r.unwrap();
        }
    }
}
