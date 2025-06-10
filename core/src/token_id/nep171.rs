use std::{fmt, str::FromStr};

use crate::token_id::{MAX_ALLOWED_TOKEN_ID_LEN, error::TokenIdError};
use near_sdk::{AccountId, AccountIdRef, near};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[near(serializers = [borsh])]
pub struct Nep171TokenId {
    account_id: AccountId,
    native_token_id: near_contract_standards::non_fungible_token::TokenId,
}

impl Nep171TokenId {
    pub fn new(
        account_id: AccountId,
        native_token_id: near_contract_standards::non_fungible_token::TokenId,
    ) -> Result<Self, TokenIdError> {
        if native_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(native_token_id.len()));
        }

        Ok(Self {
            account_id,
            native_token_id,
        })
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        self.account_id.as_ref()
    }

    pub const fn native_token_id(&self) -> &near_contract_standards::non_fungible_token::TokenId {
        &self.native_token_id
    }
}

impl std::fmt::Debug for Nep171TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.contract_id(), self.native_token_id())
    }
}

impl std::fmt::Display for Nep171TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for Nep171TokenId {
    type Err = TokenIdError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (contract_id, token_id) = data
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        if token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(token_id.len()));
        }
        Self::new(contract_id.parse()?, token_id.to_string())
    }
}

// FIXME: add tests
