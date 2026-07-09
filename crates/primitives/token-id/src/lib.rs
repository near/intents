mod error;

pub mod imt;
pub mod nep141;
pub mod nep171;
pub mod nep245;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use strum::{EnumDiscriminants, EnumIter, EnumString};

use crate::{imt::ImtTokenId, nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId};

pub use self::error::TokenIdError;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumDiscriminants, derive_more::From)]
#[strum_discriminants(
    name(TokenIdType),
    cfg_attr(
        feature = "serde",
        derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
        cfg_attr(
            feature = "abi",
            derive(::schemars::JsonSchema),
            schemars(with = "String"),
        )
    ),
    derive(strum::Display, EnumString, EnumIter),
    strum(serialize_all = "snake_case"),
    vis(pub)
)]
#[repr(u8)]
// Private: Because we need construction to go through the TokenId struct to check for length
pub enum TokenId {
    Nep141(Nep141TokenId) = 0,
    Nep171(Nep171TokenId) = 1,
    Nep245(Nep245TokenId) = 2,
    Imt(ImtTokenId) = 3,
}

impl Debug for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nep141(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep141, token_id)
            }
            Self::Nep171(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep171, token_id)
            }
            Self::Nep245(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep245, token_id)
            }
            Self::Imt(token_id) => {
                write!(f, "{}:{}", TokenIdType::Imt, token_id)
            }
        }
    }
}

impl Display for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for TokenId {
    type Err = TokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        match typ.parse()? {
            TokenIdType::Nep141 => data.parse().map(Self::Nep141),
            TokenIdType::Nep171 => data.parse().map(Self::Nep171),
            TokenIdType::Nep245 => data.parse().map(Self::Nep245),
            TokenIdType::Imt => data.parse().map(Self::Imt),
        }
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };

    impl JsonSchema for TokenId {
        fn schema_name() -> String {
            stringify!(TokenId).to_string()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            use near_account_id::AccountId;

            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: std::iter::once((
                    "examples",
                    [
                        Self::Nep141(Nep141TokenId::new("ft.near".parse::<AccountId>().unwrap())),
                        Self::Nep171(Nep171TokenId::new(
                            "nft.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                        Self::Nep245(Nep245TokenId::new(
                            "mt.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                        Self::Imt(ImtTokenId::new(
                            "imt.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                    ]
                    .map(|s| s.to_string())
                    .to_vec()
                    .into(),
                ))
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
                ..Default::default()
            }
            .into()
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use defuse_test_utils::random::make_arbitrary;
    use rstest::rstest;

    #[cfg(feature = "borsh")]
    #[rstest]
    #[trace]
    #[case::nep141("nep141:abc", "0003000000616263")]
    #[case::nep171("nep171:abc:xyz", "01030000006162630300000078797a")]
    #[case::nep245("nep245:abc:xyz", "02030000006162630300000078797a")]
    #[case::imt("imt:abc:xyz", "03030000006162630300000078797a")]
    fn roundtrip_fixed(#[case] token_id_str: &str, #[case] borsh_expected_hex: &str) {
        let token_id: TokenId = token_id_str.parse().unwrap();
        let borsh_expected = hex::decode(borsh_expected_hex).unwrap();

        let borsh_ser = borsh::to_vec(&token_id).unwrap();
        assert_eq!(borsh_ser, borsh_expected);

        let got: TokenId = borsh::from_slice(&borsh_ser).unwrap();
        assert_eq!(got, token_id);
        assert_eq!(got.to_string(), token_id_str);
    }

    #[cfg(feature = "borsh")]
    #[rstest]
    #[trace]
    fn borsh_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
        let ser = borsh::to_vec(&token_id).unwrap();
        let got: TokenId = borsh::from_slice(&ser).unwrap();
        assert_eq!(got, token_id);
    }

    #[rstest]
    #[trace]
    fn display_from_str_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
        let s = token_id.to_string();
        let got: TokenId = s.parse().unwrap();
        assert_eq!(got, token_id);
    }

    #[cfg(feature = "serde")]
    #[rstest]
    #[trace]
    fn serde_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
        let ser = serde_json::to_vec(&token_id).unwrap();
        let got: TokenId = serde_json::from_slice(&ser).unwrap();
        assert_eq!(got, token_id);
    }
}
