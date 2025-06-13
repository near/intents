pub mod error;
pub mod nep141;
pub mod nep171;
pub mod nep245;

use crate::token_id::{
    error::TokenIdError, nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId,
};
use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use near_sdk::near;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{EnumDiscriminants, EnumIter, EnumString};

const MAX_ALLOWED_TOKEN_ID_LEN: usize = 127;

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
    derive(strum::Display, EnumString, EnumIter),
    strum(serialize_all = "snake_case"),
    vis(pub)
)]
#[near(serializers = [borsh])]
// Private: Because we need construction to go through the TokenId struct to check for length
pub enum TokenId {
    Nep141(Nep141TokenId),
    Nep171(Nep171TokenId),
    Nep245(Nep245TokenId),
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
        }
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl JsonSchema for TokenId {
        fn schema_name() -> String {
            stringify!(TokenId).to_string()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: [(
                    "examples",
                    [
                        TokenId::Nep141(Nep141TokenId::new("ft.near".parse().unwrap())),
                        TokenId::Nep171(
                            Nep171TokenId::new(
                                "nft.near".parse().unwrap(),
                                "token_id1".to_string(),
                            )
                            .unwrap(),
                        ),
                        TokenId::Nep245(
                            Nep245TokenId::new("mt.near".parse().unwrap(), "token_id1".to_string())
                                .unwrap(),
                        ),
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
}

#[cfg(any(feature = "arbitrary", test))]
const _: () = {
    use arbitrary::{Arbitrary, Unstructured};
    use strum::IntoEnumIterator;
    use test_utils::arbitrary::account_id::arbitrary_account_id;

    impl<'a> Arbitrary<'a> for TokenId {
        fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
            let variants = TokenIdType::iter().collect::<Vec<_>>();
            let variant = u.choose(&variants)?;
            Ok(match variant {
                TokenIdType::Nep141 => Self::Nep141(Nep141TokenId::new(arbitrary_account_id(u)?)),
                TokenIdType::Nep171 => Self::Nep171(
                    Nep171TokenId::new(
                        arbitrary_account_id(u)?,
                        near_contract_standards::non_fungible_token::TokenId::arbitrary(u)?,
                    )
                    .unwrap(),
                ),
                TokenIdType::Nep245 => Self::Nep245(
                    Nep245TokenId::new(
                        arbitrary_account_id(u)?,
                        defuse_nep245::TokenId::arbitrary(u)?,
                    )
                    .unwrap(),
                ),
            })
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use near_sdk::{borsh, serde_json};
    use rstest::rstest;
    use test_utils::{
        arbitrary::account_id::arbitrary_account_id,
        asserts::ResultAssertsExt,
        random::{Seed, gen_random_bytes, gen_random_string, make_seedable_rng, random_seed},
    };

    #[test]
    fn fixed_data_serialization_and_deserialization() {
        let nep141 = TokenId::Nep141("abc".parse().unwrap());
        let nep171 =
            TokenId::Nep171(Nep171TokenId::new("abc".parse().unwrap(), "xyz".to_string()).unwrap());
        let nep245 =
            TokenId::Nep245(Nep245TokenId::new("abc".parse().unwrap(), "xyz".to_string()).unwrap());

        let nep141_hex_expected = "0003000000616263";
        let nep171_hex_expected = "01030000006162630300000078797a";
        let nep245_hex_expected = "02030000006162630300000078797a";

        let nep141_expected = hex::decode(nep141_hex_expected).unwrap();
        let nep171_expected = hex::decode(nep171_hex_expected).unwrap();
        let nep245_expected = hex::decode(nep245_hex_expected).unwrap();

        let nep141_deserialized = borsh::from_slice::<TokenId>(&nep141_expected).unwrap();
        let nep171_deserialized = borsh::from_slice::<TokenId>(&nep171_expected).unwrap();
        let nep245_deserialized = borsh::from_slice::<TokenId>(&nep245_expected).unwrap();

        assert_eq!(nep141_deserialized, nep141);
        assert_eq!(nep171_deserialized, nep171);
        assert_eq!(nep245_deserialized, nep245);
    }

    #[rstest]
    #[trace]
    fn serialization_back_and_forth(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        let token_id: TokenId = Arbitrary::arbitrary(&mut u).unwrap();

        let token_id_ser = borsh::to_vec(&token_id).unwrap();
        let token_id_deser: TokenId = borsh::from_slice(&token_id_ser).unwrap();

        assert_eq!(token_id_deser, token_id);
    }

    #[rstest]
    #[trace]
    fn token_id_length(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        {
            let token_id_string = gen_random_string(&mut rng, 2..1000);
            let nft_result: Result<TokenId, _> = Nep171TokenId::new(
                arbitrary_account_id(&mut u).unwrap(),
                token_id_string.clone(),
            )
            .map(Into::into);

            if token_id_string.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                nft_result.assert_err_contains("token_id is too long.");
            } else {
                nft_result.unwrap();
            }
        }

        {
            let token_id_string = gen_random_string(&mut rng, 2..1000);
            let mt_result: Result<TokenId, _> = Nep245TokenId::new(
                arbitrary_account_id(&mut u).unwrap(),
                token_id_string.clone(),
            )
            .map(Into::into);

            if token_id_string.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                mt_result.assert_err_contains("token_id is too long.");
            } else {
                mt_result.unwrap();
            }
        }
    }

    #[test]
    fn token_id_fixed_strings() {
        {
            let token_id_str = "nep141:my-token.near";
            let token_id = TokenId::from_str(token_id_str).unwrap();
            let expected_token_id = TokenId::Nep141("my-token.near".parse().unwrap());
            assert_eq!(token_id, expected_token_id);

            // Json value is the token id, but with quotes
            let json_token_id: TokenId =
                serde_json::from_str(&format!("\"{token_id_str}\"")).unwrap();
            assert_eq!(json_token_id, expected_token_id);

            // Json back and forth
            assert_eq!(
                serde_json::from_str::<TokenId>(&serde_json::to_string(&json_token_id).unwrap())
                    .unwrap(),
                expected_token_id
            );
        }
        {
            let token_id_str = "nep171:my-token.near:abc";
            let token_id = TokenId::from_str(token_id_str).unwrap();
            let expected_token_id: TokenId =
                Nep171TokenId::new("my-token.near".parse().unwrap(), "abc".to_string())
                    .unwrap()
                    .into();
            assert_eq!(token_id, expected_token_id);

            // Json value is the token id, but with quotes
            let json_token_id: TokenId =
                serde_json::from_str(&format!("\"{token_id_str}\"")).unwrap();
            assert_eq!(json_token_id, expected_token_id);

            // Json back and forth
            assert_eq!(
                serde_json::from_str::<TokenId>(&serde_json::to_string(&json_token_id).unwrap())
                    .unwrap(),
                expected_token_id
            );
        }
        {
            let token_id_str = "nep245:my-token.near:abc";
            let token_id = TokenId::from_str(token_id_str).unwrap();
            let expected_token_id: TokenId =
                Nep245TokenId::new("my-token.near".parse().unwrap(), "abc".to_string())
                    .unwrap()
                    .into();
            assert_eq!(token_id, expected_token_id);

            // Json value is the token id, but with quotes
            let json_token_id: TokenId =
                serde_json::from_str(&format!("\"{token_id_str}\"")).unwrap();
            assert_eq!(json_token_id, expected_token_id);

            // Json back and forth
            assert_eq!(
                serde_json::from_str::<TokenId>(&serde_json::to_string(&json_token_id).unwrap())
                    .unwrap(),
                expected_token_id
            );
        }
    }
}

#[cfg(test)]
mod legacy_tests;
