//! Tests that ensure that serialization to the new format adhere to the old one

use crate::token_id::{TokenId, error::TokenIdError};
use arbitrary::{Arbitrary, Unstructured};
use near_sdk::{AccountId, borsh, near};
use rstest::rstest;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{fmt, str::FromStr};
use strum::{EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};
use test_utils::{
    arbitrary::account_id::arbitrary_account_id,
    random::{Seed, gen_random_bytes, make_seedable_rng, random_seed},
};

/// A copy of the old TokenId without length checking. We have the copy here to test serialization/deserialization
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
        AccountId,
    ),
    Nep171(
        /// Contract
        AccountId,
        /// Token ID
        near_contract_standards::non_fungible_token::TokenId,
    ),
    Nep245(
        /// Contract
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

impl<'a> Arbitrary<'a> for LegacyTokenId {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let variants = LegacyTokenIdType::iter().collect::<Vec<_>>();
        let variant = u.choose(&variants)?;
        Ok(match variant {
            LegacyTokenIdType::Nep141 => Self::Nep141(arbitrary_account_id(u)?),
            LegacyTokenIdType::Nep171 => Self::Nep171(
                arbitrary_account_id(u)?,
                near_contract_standards::non_fungible_token::TokenId::arbitrary(u)?,
            ),
            LegacyTokenIdType::Nep245 => Self::Nep245(
                arbitrary_account_id(u)?,
                defuse_nep245::TokenId::arbitrary(u)?,
            ),
        })
    }
}

fn assert_eq_legacy_and_new_token_id(legacy_token_id: &LegacyTokenId, new_token_id: &TokenId) {
    match legacy_token_id {
        LegacyTokenId::Nep141(account_id) => {
            if let TokenId::Nep141(nep141) = new_token_id {
                assert_eq!(account_id, nep141.contract_id());
            } else {
                unreachable!()
            }
        }
        LegacyTokenId::Nep171(account_id, nft_token_id) => {
            if let TokenId::Nep171(nep171) = new_token_id {
                assert_eq!(account_id, nep171.contract_id());
                assert_eq!(nft_token_id, nep171.nft_token_id());
            } else {
                unreachable!()
            }
        }
        LegacyTokenId::Nep245(account_id, mt_token_id) => {
            if let TokenId::Nep245(nep245) = new_token_id {
                assert_eq!(account_id, nep245.contract_id());
                assert_eq!(mt_token_id, nep245.mt_token_id());
            } else {
                unreachable!()
            }
        }
    }
}

#[rstest]
#[trace]
fn serialization_back_and_forth_legacy_and_new(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);
    let bytes = gen_random_bytes(&mut rng, ..1000);
    let mut u = arbitrary::Unstructured::new(&bytes);

    let token_id: LegacyTokenId = Arbitrary::arbitrary(&mut u).unwrap();

    let token_id_ser = borsh::to_vec(&token_id).unwrap();
    let token_id_deser: TokenId = borsh::from_slice(&token_id_ser).unwrap();

    assert_eq_legacy_and_new_token_id(&token_id, &token_id_deser);
}
