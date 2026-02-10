mod error;

#[cfg(feature = "imt")]
pub mod imt;
#[cfg(feature = "nep141")]
pub mod nep141;
#[cfg(feature = "nep171")]
pub mod nep171;
#[cfg(feature = "nep245")]
pub mod nep245;

#[cfg(not(any(
    feature = "nep141",
    feature = "nep171",
    feature = "nep245",
    feature = "imt"
)))]
compile_error!(
    r#"At least one of these features should be enabled:
- `nep141`
- `nep171`
- `nep245`
- `imt`
"#
);

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use near_sdk::near;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{EnumDiscriminants, EnumIter, EnumString};

pub use self::error::TokenIdError;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
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
    derive_more::From,
)]
#[strum_discriminants(
    name(TokenIdType),
    derive(
        strum::Display,
        EnumString,
        EnumIter,
        SerializeDisplay,
        DeserializeFromStr
    ),
    strum(serialize_all = "snake_case"),
    cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        derive(::near_sdk::NearSchema),
        schemars(with = "String"),
    ),
    vis(pub)
)]
#[near(serializers = [borsh(use_discriminant=true)])]
#[repr(u8)]
// Private: Because we need construction to go through the TokenId struct to check for length
pub enum TokenId {
    #[cfg(feature = "nep141")]
    Nep141(crate::nep141::Nep141TokenId) = 0,
    #[cfg(feature = "nep171")]
    Nep171(crate::nep171::Nep171TokenId) = 1,
    #[cfg(feature = "nep245")]
    Nep245(crate::nep245::Nep245TokenId) = 2,
    #[cfg(feature = "imt")]
    Imt(crate::imt::ImtTokenId) = 3,
}

impl Debug for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep141, token_id)
            }
            #[cfg(feature = "nep171")]
            Self::Nep171(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep171, token_id)
            }
            #[cfg(feature = "nep245")]
            Self::Nep245(token_id) => {
                write!(f, "{}:{}", TokenIdType::Nep245, token_id)
            }
            #[cfg(feature = "imt")]
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
            #[cfg(feature = "nep141")]
            TokenIdType::Nep141 => data.parse().map(Self::Nep141),
            #[cfg(feature = "nep171")]
            TokenIdType::Nep171 => data.parse().map(Self::Nep171),
            #[cfg(feature = "nep245")]
            TokenIdType::Nep245 => data.parse().map(Self::Nep245),
            #[cfg(feature = "imt")]
            TokenIdType::Imt => data.parse().map(Self::Imt),
        }
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
const _: () = {
    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };

    impl JsonSchema for TokenId {
        fn schema_name() -> String {
            stringify!(TokenId).to_string()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            use near_sdk::AccountId;

            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: [(
                    "examples",
                    [
                        #[cfg(feature = "nep141")]
                        TokenId::Nep141(crate::nep141::Nep141TokenId::new(
                            "ft.near".parse::<AccountId>().unwrap(),
                        )),
                        #[cfg(feature = "nep171")]
                        TokenId::Nep171(crate::nep171::Nep171TokenId::new(
                            "nft.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                        #[cfg(feature = "nep245")]
                        TokenId::Nep245(crate::nep245::Nep245TokenId::new(
                            "mt.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                        #[cfg(feature = "imt")]
                        TokenId::Imt(crate::imt::ImtTokenId::new(
                            "imt.near".parse::<AccountId>().unwrap(),
                            "token_id1",
                        )),
                    ]
                    .map(|s| s.to_string())
                    .to_vec()
                    .into(),
                )]
                .into_iter()
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
    use near_sdk::{borsh, serde_json};
    use rstest::rstest;

    #[rstest]
    #[trace]
    #[cfg_attr(feature = "nep141", case("nep141:abc", "0003000000616263"))]
    #[cfg_attr(
        feature = "nep171",
        case("nep171:abc:xyz", "01030000006162630300000078797a")
    )]
    #[cfg_attr(
        feature = "nep245",
        case("nep245:abc:xyz", "02030000006162630300000078797a")
    )]
    #[cfg_attr(feature = "imt", case("imt:abc:xyz", "03030000006162630300000078797a"))]
    fn roundtrip_fixed(#[case] token_id_str: &str, #[case] borsh_expected_hex: &str) {
        let token_id: TokenId = token_id_str.parse().unwrap();
        let borsh_expected = hex::decode(borsh_expected_hex).unwrap();

        let borsh_ser = borsh::to_vec(&token_id).unwrap();
        assert_eq!(borsh_ser, borsh_expected);

        let got: TokenId = borsh::from_slice(&borsh_ser).unwrap();
        assert_eq!(got, token_id);
        assert_eq!(got.to_string(), token_id_str);
    }

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

    #[rstest]
    #[trace]
    fn serde_roundtrip(#[from(make_arbitrary)] token_id: TokenId) {
        let ser = serde_json::to_vec(&token_id).unwrap();
        let got: TokenId = serde_json::from_slice(&ser).unwrap();
        assert_eq!(got, token_id);
    }
}
