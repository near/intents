use std::{borrow::Cow, collections::BTreeMap};

use derive_more::From;
use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::Params;

#[near(event_json(
    // TODO
    standard = "escrow",
))]
#[derive(Debug, Clone, From)]
pub enum Event<'a> {
    #[event_version("0.1.0")]
    Create(Cow<'a, Params>),

    #[event_version("0.1.0")]
    AddSrc(AddSrcEvent),

    #[event_version("0.1.0")]
    Fill(FillEvent<'a>),

    // TODO: enrich with:
    // closed_by: maker/taker/authority
    #[event_version("0.1.0")]
    Close,

    #[event_version("0.1.0")]
    Cleanup,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct AddSrcEvent {
    pub maker: AccountId,

    #[serde_as(as = "DisplayFromStr")]
    pub src_amount_added: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub src_remaining: u128,
    // TODO: include price
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillEvent<'a> {
    pub taker: Cow<'a, AccountIdRef>,

    // TODO: src_token
    // TODO: dst_token
    #[serde_as(as = "DisplayFromStr")]
    pub src_amount: u128,
    // TODO: is it how much will the maker get? or how much taker spent?
    #[serde_as(as = "DisplayFromStr")]
    pub dst_amount: u128,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_receiver_id: Option<Cow<'a, AccountIdRef>>,

    #[serde_as(as = "BTreeMap<_, DisplayFromStr>")]
    pub dst_fees_collected: BTreeMap<Cow<'a, AccountIdRef>, u128>,
    // TODO: how much dst will maker receive?
}

pub trait EscrowIntentEmit<'a>: Into<Event<'a>> {
    #[inline]
    fn emit(self) {
        Event::emit(&self.into());
    }
}

impl<'a, T> EscrowIntentEmit<'a> for T where T: Into<Event<'a>> {}
