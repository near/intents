use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use near_account_id::ParseAccountError;
use near_sdk::{AccountId, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{EnumDiscriminants, EnumString};
use thiserror::Error as ThisError;

const MAX_ALLOWED_TOKEN_ID_LEN: usize = 127;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[near(serializers = [borsh])]
pub struct TokenId {
    token_id: TokenIdHolder,
}

impl TokenId {
    pub const fn make_nep141(account_id: AccountId) -> Self {
        Self {
            token_id: TokenIdHolder::Nep141(account_id),
        }
    }

    pub fn make_nep171(
        account_id: AccountId,
        native_token_id: near_contract_standards::non_fungible_token::TokenId,
    ) -> Result<Self, TokenIdError> {
        if native_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(native_token_id.len()));
        }

        Ok(Self {
            token_id: TokenIdHolder::Nep171(account_id, native_token_id),
        })
    }

    pub fn make_nep245(
        account_id: AccountId,
        defuse_token_id: defuse_nep245::TokenId,
    ) -> Result<Self, TokenIdError> {
        if defuse_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(defuse_token_id.len()));
        }

        Ok(Self {
            token_id: TokenIdHolder::Nep245(account_id, defuse_token_id),
        })
    }

    pub const fn which(&self) -> TokenIdType {
        match self.token_id {
            TokenIdHolder::Nep141(..) => TokenIdType::Nep141,
            TokenIdHolder::Nep171(..) => TokenIdType::Nep171,
            TokenIdHolder::Nep245(..) => TokenIdType::Nep245,
        }
    }
}

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
    name(TokenIdType),
    derive(strum::Display, EnumString),
    strum(serialize_all = "snake_case"),
    vis(pub)
)]
#[near(serializers = [borsh])]
// Private: Because we need construction to go through the TokenId struct to check for length
enum TokenIdHolder {
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

impl Debug for TokenIdHolder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nep141(contract_id) => {
                write!(f, "{}:{}", TokenIdType::Nep141, contract_id)
            }
            Self::Nep171(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep171, contract_id, token_id)
            }
            Self::Nep245(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep245, contract_id, token_id)
            }
        }
    }
}

impl Display for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.token_id, f)
    }
}

impl Display for TokenIdHolder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for TokenIdHolder {
    type Err = TokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(match typ.parse()? {
            TokenIdType::Nep141 => Self::Nep141(data.parse()?),
            TokenIdType::Nep171 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                if token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                    return Err(TokenIdError::TokenIdTooLarge(token_id.len()));
                }
                Self::Nep171(contract_id.parse()?, token_id.to_string())
            }
            TokenIdType::Nep245 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                if token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                    return Err(TokenIdError::TokenIdTooLarge(token_id.len()));
                }
                Self::Nep245(contract_id.parse()?, token_id.to_string())
            }
        })
    }
}

impl FromStr for TokenId {
    type Err = TokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token_id: TokenIdHolder = s.parse()?;
        Ok(Self { token_id })
    }
}

#[derive(Debug, ThisError)]
pub enum TokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
    #[error("Token id provided is too large. Given: {0}. Max: {MAX_ALLOWED_TOKEN_ID_LEN}")]
    TokenIdTooLarge(usize),
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
            stringify!(TokenIdHolder).to_string()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: [(
                    "examples",
                    [
                        Self {
                            token_id: TokenIdHolder::Nep141("ft.near".parse().unwrap()),
                        },
                        Self {
                            token_id: TokenIdHolder::Nep171(
                                "nft.near".parse().unwrap(),
                                "token_id1".to_string(),
                            ),
                        },
                        Self {
                            token_id: TokenIdHolder::Nep245(
                                "mt.near".parse().unwrap(),
                                "token_id1".to_string(),
                            ),
                        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::{Arbitrary, Unstructured};
    use near_sdk::borsh;
    use rstest::rstest;
    use test_utils::{
        asserts::ResultAssertsExt,
        random::{Seed, gen_random_bytes, gen_random_string, make_seedable_rng, random_seed},
    };

    #[allow(clippy::as_conversions)]
    fn arbitrary_account_id(u: &mut Unstructured<'_>) -> arbitrary::Result<AccountId> {
        if u.arbitrary()? {
            // Named account id
            let len = u.int_in_range(3..=20)?;
            let s: String = (0..len)
                .map(|_| {
                    let c = u.int_in_range(0..=35)?;
                    Ok(match c {
                        0..=25 => (b'a' + c) as char,
                        26..=35 => (b'0' + (c - 26)) as char,
                        _ => unreachable!(),
                    })
                })
                .collect::<arbitrary::Result<_>>()?;
            let s = s + ".near";
            s.parse().map_err(|_| arbitrary::Error::IncorrectFormat)
        } else {
            // Explicit numeric account id
            let len = 64;
            let s: String = (0..len)
                .map(|_| {
                    let c = u.int_in_range(0..=15)?;
                    Ok(match c {
                        0..=9 => (b'0' + c) as char,
                        10..=15 => (b'a' + (c - 10)) as char,
                        _ => unreachable!(),
                    })
                })
                .collect::<arbitrary::Result<_>>()?;
            s.parse().map_err(|_| arbitrary::Error::IncorrectFormat)
        }
    }

    impl<'a> Arbitrary<'a> for TokenIdHolder {
        fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
            let variant = u.int_in_range(0..=2)?;
            Ok(match variant {
                0 => Self::Nep141(arbitrary_account_id(u)?),
                1 => Self::Nep171(
                    arbitrary_account_id(u)?,
                    near_contract_standards::non_fungible_token::TokenId::arbitrary(u)?,
                ),
                2 => Self::Nep245(
                    arbitrary_account_id(u)?,
                    defuse_nep245::TokenId::arbitrary(u)?,
                ),
                _ => unreachable!(),
            })
        }
    }

    impl<'a> Arbitrary<'a> for TokenId {
        fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
            Ok(Self {
                token_id: TokenIdHolder::arbitrary(u)?,
            })
        }
    }

    #[test]
    fn holder_fixed_data_serialization_and_deserialization() {
        let nep141 = TokenIdHolder::Nep141("abc".parse().unwrap());
        let nep171 = TokenIdHolder::Nep171("abc".parse().unwrap(), "xyz".to_string());
        let nep245 = TokenIdHolder::Nep245("abc".parse().unwrap(), "xyz".to_string());

        let nep141_hex_expected = "0003000000616263";
        let nep171_hex_expected = "01030000006162630300000078797a";
        let nep245_hex_expected = "02030000006162630300000078797a";

        let nep141_expected = hex::decode(nep141_hex_expected).unwrap();
        let nep171_expected = hex::decode(nep171_hex_expected).unwrap();
        let nep245_expected = hex::decode(nep245_hex_expected).unwrap();

        let nep141_deserialized = borsh::from_slice::<TokenIdHolder>(&nep141_expected).unwrap();
        let nep171_deserialized = borsh::from_slice::<TokenIdHolder>(&nep171_expected).unwrap();
        let nep245_deserialized = borsh::from_slice::<TokenIdHolder>(&nep245_expected).unwrap();

        assert_eq!(nep141_deserialized, nep141);
        assert_eq!(nep171_deserialized, nep171);
        assert_eq!(nep245_deserialized, nep245);
    }

    #[test]
    fn token_id_fixed_data_serialization_and_deserialization() {
        let nep141 = TokenId::make_nep141("abc".parse().unwrap());
        let nep171 = TokenId::make_nep171("abc".parse().unwrap(), "xyz".to_string()).unwrap();
        let nep245 = TokenId::make_nep245("abc".parse().unwrap(), "xyz".to_string()).unwrap();

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
    fn holder_serialization_back_and_forth(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        let token_id: TokenIdHolder = Arbitrary::arbitrary(&mut u).unwrap();

        let token_id_ser = borsh::to_vec(&token_id).unwrap();
        let token_id_deser: TokenIdHolder = borsh::from_slice(&token_id_ser).unwrap();

        assert_eq!(token_id_deser, token_id);
    }

    #[rstest]
    #[trace]
    fn token_id_serialization_back_and_forth(random_seed: Seed) {
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
            let nft_result = TokenId::make_nep171(
                arbitrary_account_id(&mut u).unwrap(),
                token_id_string.clone(),
            );
            if token_id_string.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                nft_result.assert_err_contains("Token id provided is too large.");
            } else {
                nft_result.unwrap();
            }
        }

        {
            let token_id_string = gen_random_string(&mut rng, 2..1000);
            let mt_result = TokenId::make_nep245(
                arbitrary_account_id(&mut u).unwrap(),
                token_id_string.clone(),
            );
            if token_id_string.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                mt_result.assert_err_contains("Token id provided is too large.");
            } else {
                mt_result.unwrap();
            }
        }
    }
}
