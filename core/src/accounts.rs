use std::collections::BTreeMap;

use defuse_crypto::PublicKey;
use defuse_serde_utils::base64::Base64;
use near_sdk::{AccountIdRef, near};
use serde_with::{DisplayFromStr, serde_as};
use std::{borrow::Cow, collections::BTreeSet};

use crate::{Nonce, Salt, amounts::Amounts};

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

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct NonceEvent {
    #[serde_as(as = "Base64")]
    pub nonce: Nonce,
}

impl NonceEvent {
    #[inline]
    pub const fn new(nonce: Nonce) -> Self {
        Self { nonce }
    }
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SaltRotationEvent {
    pub current: Salt,
    pub invalidated: BTreeSet<Salt>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferEvent<'a> {
    pub receiver_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: Amounts,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Cow<'a, Option<String>>,
}
