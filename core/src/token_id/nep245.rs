use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::token_id::{MAX_ALLOWED_TOKEN_ID_LEN, error::TokenIdError};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
#[must_use]
pub struct Nep245TokenId {
    contract_id: AccountId,
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
    use arbitrary::Arbitrary;
    use rstest::rstest;
    use test_utils::{
        arbitrary::account_id::arbitrary_account_id,
        asserts::ResultAssertsExt,
        random::{Seed, gen_random_bytes, gen_random_string, make_seedable_rng, random_seed},
    };

    #[rstest]
    #[trace]
    fn to_from_string(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        let account_id = arbitrary_account_id(&mut u).unwrap();
        let native_token_id =
            near_contract_standards::non_fungible_token::TokenId::arbitrary(&mut u).unwrap();
        let token_id = Nep245TokenId::new(account_id.clone(), native_token_id.clone()).unwrap();

        assert_eq!(
            token_id.to_string(),
            format!("{account_id}:{native_token_id}")
        );

        assert_eq!(
            Nep245TokenId::from_str(&token_id.to_string()).unwrap(),
            token_id
        );
    }

    #[rstest]
    #[trace]
    fn from_string_length_limit(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        let token_id_string = gen_random_string(&mut rng, 2..1000);
        let nft_result = Nep245TokenId::new(
            arbitrary_account_id(&mut u).unwrap(),
            token_id_string.clone(),
        );
        if token_id_string.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            nft_result.assert_err_contains("token_id is too long.");
        } else {
            let _ = nft_result.unwrap();
        }
    }

    #[rstest]
    #[trace]
    fn fixed_from_string() {
        let account_id = AccountId::from_str("my-token.near").unwrap();
        let native_token_id = "abc";
        let token_id = Nep245TokenId::new(account_id, native_token_id.to_string()).unwrap();

        let expected_token_id_str = "my-token.near:abc";
        assert_eq!(token_id.to_string(), expected_token_id_str);
        assert_eq!(token_id, expected_token_id_str.parse().unwrap());
    }
}
