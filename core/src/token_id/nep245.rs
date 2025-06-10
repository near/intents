use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};

use crate::token_id::{MAX_ALLOWED_TOKEN_ID_LEN, error::TokenIdError};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[near(serializers = [borsh])]
pub struct Nep245TokenId {
    account_id: AccountId,
    defuse_token_id: defuse_nep245::TokenId,
}

impl Nep245TokenId {
    pub fn new(
        account_id: AccountId,
        defuse_token_id: defuse_nep245::TokenId,
    ) -> Result<Self, TokenIdError> {
        if defuse_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(defuse_token_id.len()));
        }

        Ok(Self {
            account_id,
            defuse_token_id,
        })
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        self.account_id.as_ref()
    }

    pub const fn defuse_token_id(&self) -> &defuse_nep245::TokenId {
        &self.defuse_token_id
    }
}

impl std::fmt::Debug for Nep245TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.contract_id(), self.defuse_token_id())
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
        if token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
            return Err(TokenIdError::TokenIdTooLarge(token_id.len()));
        }
        Self::new(contract_id.parse()?, token_id.to_string())
    }
}

// FIXME: add tests
