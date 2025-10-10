use defuse_crypto::PublicKey;
use defuse_serde_utils::base64::Base64;
use near_sdk::{AccountIdRef, near};
use serde_with::serde_as;
use std::{borrow::Cow, collections::BTreeSet};

use crate::{Nonce, Salt};

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountEvent<'a, T> {
    pub account_id: Cow<'a, AccountIdRef>,

    #[serde(flatten)]
    pub event: T,
}

impl<T: 'static> AccountEvent<'_, T> {
    pub fn into_owned(self) -> AccountEvent<'static, T> {
        AccountEvent {
            account_id: Cow::Owned(self.account_id.into_owned()),
            event: self.event,
        }
    }
}

// For AccountEvent with PublicKeyEvent
impl<'a> AccountEvent<'a, PublicKeyEvent<'a>> {
    pub fn into_owned_public_key(self) -> AccountEvent<'static, PublicKeyEvent<'static>> {
        AccountEvent {
            account_id: Cow::Owned(self.account_id.into_owned()),
            event: self.event.into_owned(),
        }
    }
}

// For AccountEvent with TokenDiffEvent
impl<'a> AccountEvent<'a, crate::intents::token_diff::TokenDiffEvent<'a>> {
    pub fn into_owned_token_diff(
        self,
    ) -> AccountEvent<'static, crate::intents::token_diff::TokenDiffEvent<'static>> {
        AccountEvent {
            account_id: Cow::Owned(self.account_id.into_owned()),
            event: crate::intents::token_diff::TokenDiffEvent {
                diff: Cow::Owned(self.event.diff.into_owned()),
                fees_collected: self.event.fees_collected,
            },
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyEvent<'a> {
    pub public_key: Cow<'a, PublicKey>,
}

impl PublicKeyEvent<'_> {
    #[inline]
    pub fn into_owned(self) -> PublicKeyEvent<'static> {
        PublicKeyEvent {
            public_key: Cow::Owned(self.public_key.into_owned()),
        }
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaltRotationEvent {
    pub current: Salt,
    pub invalidated: BTreeSet<Salt>,
}
