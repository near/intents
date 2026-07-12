use std::{borrow::Cow, collections::BTreeSet};

use near_sdk::AccountIdRef;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::{Nonce, Salt, public_key::PublicKey};

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyEvent<'a> {
    pub public_key: Cow<'a, PublicKey>,
}

#[serde_as]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaltRotationEvent {
    pub current: Salt,
    pub invalidated: BTreeSet<Salt>,
}
