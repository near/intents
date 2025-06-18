//! Tests that ensure that serialization to the new format adhere to the old one

use crate::token_id::{TokenId, error::TokenIdError};
use arbitrary_with::{Arbitrary, As};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_test_utils::random::make_arbitrary;
use near_sdk::{AccountId, borsh, near};
use rstest::rstest;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{fmt, str::FromStr};
use strum::{EnumDiscriminants, EnumIter, EnumString};

/// A copy of the old TokenId without length checking. We have the copy here to test serialization/deserialization
#[cfg_attr(any(feature = "arbitrary", test), derive(Arbitrary))]
#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumDiscriminants,
    SerializeDisplay,
    DeserializeFromStr,
)]
#[strum_discriminants(
    name(LegacyTokenIdType),
    derive(strum::Display, EnumString, EnumIter),
    strum(serialize_all = "snake_case")
)]
#[near(serializers = [borsh])]
enum LegacyTokenId {
    Nep141(
        /// Contract
        #[cfg_attr(
            any(feature = "arbitrary", test),
            arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
        )]
        AccountId,
    ),
    Nep171(
        /// Contract
        #[cfg_attr(
            any(feature = "arbitrary", test),
            arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
        )]
        AccountId,
        /// Token ID
        near_contract_standards::non_fungible_token::TokenId,
    ),
    Nep245(
        /// Contract
        #[cfg_attr(
            any(feature = "arbitrary", test),
            arbitrary(with = As::<ArbitraryAccountId>::arbitrary),
        )]
        AccountId,
        /// Token ID
        defuse_nep245::TokenId,
    ),
}

impl std::fmt::Debug for LegacyTokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nep141(contract_id) => {
                write!(f, "{}:{}", LegacyTokenIdType::Nep141, contract_id)
            }
            Self::Nep171(contract_id, token_id) => {
                write!(
                    f,
                    "{}:{}:{}",
                    LegacyTokenIdType::Nep171,
                    contract_id,
                    token_id
                )
            }
            Self::Nep245(contract_id, token_id) => {
                write!(
                    f,
                    "{}:{}:{}",
                    LegacyTokenIdType::Nep245,
                    contract_id,
                    token_id
                )
            }
        }
    }
}

impl std::fmt::Display for LegacyTokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for LegacyTokenId {
    type Err = TokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(match typ.parse()? {
            LegacyTokenIdType::Nep141 => Self::Nep141(data.parse()?),
            LegacyTokenIdType::Nep171 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep171(contract_id.parse()?, token_id.to_string())
            }
            LegacyTokenIdType::Nep245 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep245(contract_id.parse()?, token_id.to_string())
            }
        })
    }
}

impl From<TokenId> for LegacyTokenId {
    fn from(token_id: TokenId) -> Self {
        match token_id {
            TokenId::Nep141(token_id) => Self::Nep141(token_id.into_contract_id()),
            TokenId::Nep171(token_id) => {
                let (contract_id, nft_token_id) = token_id.into_contract_id_and_nft_token_id();
                Self::Nep171(contract_id, nft_token_id)
            }
            TokenId::Nep245(token_id) => {
                let (contract_id, mt_token_id) = token_id.into_contract_id_and_mt_token_id();
                Self::Nep245(contract_id, mt_token_id)
            }
        }
    }
}

#[rstest]
#[trace]
fn borsh_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
    let legacy_token_id: LegacyTokenId = token_id.clone().into();

    let ser = borsh::to_vec(&token_id).unwrap();
    assert_eq!(ser, borsh::to_vec(&legacy_token_id).unwrap());

    let got: TokenId = borsh::from_slice(&ser).unwrap();
    let legacy_got: LegacyTokenId = borsh::from_slice(&ser).unwrap();
    assert_eq!(got, token_id);
    assert_eq!(legacy_got, legacy_token_id);
}

#[rstest]
#[trace]
fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
    let legacy_token_id: LegacyTokenId = token_id.clone().into();

    let ser = token_id.to_string();
    assert_eq!(ser, legacy_token_id.to_string());

    let got: TokenId = ser.parse().unwrap();
    let legacy_got: LegacyTokenId = ser.parse().unwrap();
    assert_eq!(got, token_id);
    assert_eq!(legacy_got, legacy_token_id);
}
