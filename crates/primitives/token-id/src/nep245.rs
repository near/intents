pub use defuse_nep245::TokenId;

use std::{fmt, str::FromStr};

use near_account_id::AccountId;

use crate::{TokenIdType, error::TokenIdError};

#[cfg_attr(any(feature = "arbitrary", test), derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "abi",
        derive(::schemars::JsonSchema),
        schemars(with = "String")
    )
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nep245TokenId {
    pub contract_id: AccountId,

    pub mt_token_id: TokenId,
}

impl Nep245TokenId {
    pub fn new(contract_id: impl Into<AccountId>, mt_token_id: impl Into<TokenId>) -> Self {
        Self {
            contract_id: contract_id.into(),
            mt_token_id: mt_token_id.into(),
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
        Ok(Self::new(contract_id.parse::<AccountId>()?, token_id))
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
