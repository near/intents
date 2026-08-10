use defuse_nep245::TokenId;
use defuse_serde_utils::AsCowSlice;
use near_sdk::{AccountIdRef, near, serde::Deserialize};
use serde_with::{DisplayFromStr, serde_as};
use std::borrow::Cow;

#[must_use = "make sure to `.emit()` this event"]
#[serde_as]
#[near(event_json(standard = "logger"))]
#[derive(Debug, Clone, Deserialize)]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    MtDeposit {
        token: Cow<'a, AccountIdRef>,
        sender_id: Cow<'a, AccountIdRef>,
        previous_owner_ids: Cow<'a, [Cow<'a, AccountIdRef>]>,
        token_ids: Cow<'a, [TokenId]>,
        #[serde_as(as = "AsCowSlice<DisplayFromStr>")]
        amounts: Cow<'a, [u128]>,
        msg: Cow<'a, str>,
        #[serde_as(as = "DisplayFromStr")]
        nonce: u128,
    },
}
