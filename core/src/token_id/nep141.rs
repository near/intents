use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::token_id::error::TokenIdError;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr)]
#[near(serializers = [borsh])]
#[must_use]
pub struct Nep141TokenId {
    contract_id: AccountId,
}

impl Nep141TokenId {
    pub const fn new(contract_id: AccountId) -> Self {
        Self { contract_id }
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        self.contract_id.as_ref()
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
    use rstest::rstest;
    use test_utils::{
        arbitrary::account_id::arbitrary_account_id,
        random::{Seed, gen_random_bytes, make_seedable_rng, random_seed},
    };

    #[rstest]
    #[trace]
    fn to_from_string(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        let account_id = arbitrary_account_id(&mut u).unwrap();
        let token_id = Nep141TokenId::new(account_id.clone());

        assert_eq!(token_id.to_string(), account_id.to_string());

        assert_eq!(
            Nep141TokenId::from_str(&token_id.to_string()).unwrap(),
            token_id
        );
    }
}
