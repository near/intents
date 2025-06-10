use std::{fmt, str::FromStr};

use near_sdk::{AccountId, AccountIdRef, near};

use crate::token_id::error::TokenIdError;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[near(serializers = [borsh])]
pub struct Nep141TokenId {
    account_id: AccountId,
}

impl Nep141TokenId {
    pub const fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }

    pub fn contract_id(&self) -> &AccountIdRef {
        self.account_id.as_ref()
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
            account_id: data.parse()?,
        })
    }
}

// FIXME: add tests
