use std::{borrow::Cow, collections::BTreeMap};

use defuse_token_id::TokenId;
use derive_more::From;
use near_sdk::{AccountId, AccountIdRef, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{Params, price::Price, tokens::Sent};

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

    // TODO: emit
    #[event_version("0.1.0")]
    MakerLost(MakerLost),

    #[event_version("0.1.0")]
    MakerLostFound {
        // TODO
    },

    // TODO: enrich with:
    // closed_by: maker/taker
    #[event_version("0.1.0")]
    Close { reason: CloseReason },

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
    pub maker: Cow<'a, AccountIdRef>,
    pub taker: Cow<'a, AccountIdRef>,

    pub src_token: Cow<'a, TokenId>,
    pub dst_token: Cow<'a, TokenId>,

    pub taker_price: Price,
    pub maker_price: Price,

    #[serde_as(as = "DisplayFromStr")]
    pub taker_dst_in: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub taker_dst_used: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub src_out: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub maker_dst_out: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_remaining: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taker_receive_src_to: Option<Cow<'a, AccountIdRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker_receive_dst_to: Option<Cow<'a, AccountIdRef>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_dst_fees: Option<ProtocolFeesCollected<'a>>,

    #[serde_as(as = "BTreeMap<_, DisplayFromStr>")]
    pub integrator_dst_fees: BTreeMap<Cow<'a, AccountIdRef>, u128>,
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
#[derive(Debug, Clone)]
pub struct ProtocolFeesCollected<'a> {
    #[serde_as(as = "DisplayFromStr")]
    pub fee: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub surplus: u128,
    pub collector: Cow<'a, AccountIdRef>,
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
pub struct MakerLost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<Sent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dst: Option<Sent>,
}

#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    DeadlineExpired,
    ByMaker,
    BySingleTaker,
}

pub trait EscrowIntentEmit<'a>: Into<Event<'a>> {
    #[inline]
    fn emit(self) {
        Event::emit(&self.into());
    }
}

impl<'a, T> EscrowIntentEmit<'a> for T where T: Into<Event<'a>> {}
