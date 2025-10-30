use std::{borrow::Cow, collections::BTreeMap};

use derive_more::From;
use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{FixedParams, Params};

#[near(event_json(
    // TODO
    standard = "escrow",
))]
#[derive(Debug, Clone, From)]
pub enum EscrowEvent<'a> {
    #[event_version("0.1.0")]
    Created(CreatedEvent<'a>),

    #[event_version("0.1.0")]
    AddSrc(AddSrcEvent),

    #[event_version("0.1.0")]
    Fill(FillEvent<'a>),

    #[event_version("0.1.0")]
    Close,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct CreatedEvent<'a> {
    #[serde(flatten)]
    pub fixed: Cow<'a, FixedParams>,
    #[serde(flatten)]
    pub params: Cow<'a, Params>,
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

    #[serde_as(as = "DisplayFromStr")]
    pub src_amount: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub dst_amount: u128,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_receiver_id: Option<Cow<'a, AccountIdRef>>,

    #[serde_as(as = "BTreeMap<_, DisplayFromStr>")]
    pub dst_fees_collected: BTreeMap<Cow<'a, AccountIdRef>, u128>,
    // TODO
}

pub trait EscrowIntentEmit<'a>: Into<EscrowEvent<'a>> {
    #[inline]
    fn emit(self) {
        EscrowEvent::emit(&self.into());
    }
}

impl<'a, T> EscrowIntentEmit<'a> for T where T: Into<EscrowEvent<'a>> {}
