//! Bindings to [`Wallet`](defuse_wallet::contract::Wallet) contract

use std::{borrow::Cow, collections::BTreeSet};

use defuse_wallet::{Request, RequestMessage, Timestamp};
use derive_more::From;
use near_kit::{AccountId, AccountIdRef};
use serde::Serialize;

#[near_kit::contract]
pub trait WalletContract {
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

impl<'a, P> From<(RequestMessage, P)> for WExecuteSignedArgs<'a>
where
    P: Into<Cow<'a, str>>,
{
    #[inline]
    fn from((msg, proof): (RequestMessage, P)) -> Self {
        Self {
            msg: Cow::Owned(msg),
            proof: proof.into(),
        }
    }
}

impl<'a, P> From<(&'a RequestMessage, P)> for WExecuteSignedArgs<'a>
where
    P: Into<Cow<'a, str>>,
{
    #[inline]
    fn from((msg, proof): (&'a RequestMessage, P)) -> Self {
        Self {
            msg: Cow::Borrowed(msg),
            proof: proof.into(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, From)]
#[from(forward)]
pub struct WIsExtensionEnabledArgs<'a> {
    pub account_id: Cow<'a, AccountIdRef>,
}
