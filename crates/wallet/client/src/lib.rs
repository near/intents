use std::{borrow::Cow, collections::BTreeSet};

use defuse_wallet_core::{Request, RequestMessage, Timestamp};
use near_kit::{AccountId, AccountIdRef};
use serde::Serialize;

#[near_kit::contract]
pub trait Wallet {
    #[call]
    fn w_execute_signed(&mut self, args: WExecuteSignedArgs<'_>);

    #[call]
    fn w_execute_extension(&mut self, args: WExecuteExtensionArgs<'_>);

    fn w_subwallet_id(&self) -> u32;

    fn w_is_signature_allowed(&self) -> bool;

    fn w_public_key(&self) -> String;

    fn w_is_extension_enabled(&self, args: WIsExtensionEnabledArgs<'_>) -> bool;

    fn w_extensions(&self) -> BTreeSet<AccountId>;

    fn w_timeout_secs(&self) -> u32;

    fn w_last_cleaned_at(&self) -> Timestamp;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WExecuteSignedArgs<'a> {
    pub msg: Cow<'a, RequestMessage>,
    pub proof: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WExecuteExtensionArgs<'a> {
    pub request: Cow<'a, Request>,
}

impl<'a> From<&'a Request> for WExecuteExtensionArgs<'a> {
    #[inline]
    fn from(request: &'a Request) -> Self {
        Self {
            request: Cow::Borrowed(request),
        }
    }
}

impl<'a> From<Cow<'a, Request>> for WExecuteExtensionArgs<'a> {
    #[inline]
    fn from(request: Cow<'a, Request>) -> Self {
        Self { request }
    }
}

impl From<Request> for WExecuteExtensionArgs<'_> {
    #[inline]
    fn from(request: Request) -> Self {
        Self {
            request: Cow::Owned(request),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WIsExtensionEnabledArgs<'a> {
    pub account_id: Cow<'a, AccountIdRef>,
}

impl<'a> From<&'a AccountIdRef> for WIsExtensionEnabledArgs<'a> {
    #[inline]
    fn from(account_id: &'a AccountIdRef) -> Self {
        Self {
            account_id: Cow::Borrowed(account_id),
        }
    }
}

impl<'a> From<Cow<'a, AccountIdRef>> for WIsExtensionEnabledArgs<'a> {
    #[inline]
    fn from(account_id: Cow<'a, AccountIdRef>) -> Self {
        Self { account_id }
    }
}

impl From<AccountId> for WIsExtensionEnabledArgs<'_> {
    #[inline]
    fn from(account_id: AccountId) -> Self {
        Self {
            account_id: Cow::Owned(account_id),
        }
    }
}
