use std::borrow::Cow;

use defuse_crypto::PublicKey;
use near_sdk::{AccountIdRef, near};

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct AccountEvent<'a, T> {
    pub account_id: Cow<'a, AccountIdRef>,

    #[serde(flatten)]
    pub event: T,
}

impl<T> AccountEvent<'_, T> {
    pub fn into_owned(self) -> AccountEvent<'static, T> {
        AccountEvent {
            account_id: Cow::Owned(self.account_id.into_owned()),
            event: self.event,
        }
    }
}

impl<'a, T> AccountEvent<'a, T> {
    #[inline]
    pub fn new(account_id: impl Into<Cow<'a, AccountIdRef>>, event: T) -> Self {
        Self {
            account_id: account_id.into(),
            event,
        }
    }
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct PublicKeyEvent<'a> {
    pub public_key: Cow<'a, PublicKey>,
}
