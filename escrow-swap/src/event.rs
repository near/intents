use std::{borrow::Cow, collections::BTreeMap};

use derive_more::From;
use near_sdk::{AccountIdRef, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{Params, price::Price, token_id::TokenId};

#[near(event_json(standard = "escrow-swap"))]
#[derive(Debug, Clone, From)]
pub enum Event<'a> {
    // TODO: remove due to state_init()
    #[event_version("0.1.0")]
    Init(Cow<'a, Params>),

    #[event_version("0.1.0")]
    Funded(FundedEvent<'a>),

    #[event_version("0.1.0")]
    Fill(FillEvent<'a>),

    #[event_version("0.1.0")]
    Closed { reason: CloseReason },

    #[from(skip)]
    #[event_version("0.1.0")]
    MakerRefunded(MakerSent),

    #[from(skip)]
    #[event_version("0.1.0")]
    MakerLost(MakerSent),

    #[event_version("0.1.0")]
    Cleanup,
}

#[must_use = "make sure to `.emit()` this event"]
// #[cfg_attr(
//     all(feature = "abi", not(target_arch = "wasm32")),
//     serde_as(schemars = true)
// )]
// #[cfg_attr(
//     not(all(feature = "abi", not(target_arch = "wasm32"))),
//     serde_as(schemars = false)
// )]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FundedEvent<'a> {
    pub params: Cow<'a, Params>,

    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_added: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_remaining: u128,
}

#[must_use = "make sure to `.emit()` this event"]
// #[cfg_attr(
//     all(feature = "abi", not(target_arch = "wasm32")),
//     serde_as(schemars = true)
// )]
// #[cfg_attr(
//     not(all(feature = "abi", not(target_arch = "wasm32"))),
//     serde_as(schemars = false)
// )]
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
    pub taker_src_out: u128,
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

// #[cfg_attr(
//     all(feature = "abi", not(target_arch = "wasm32")),
//     serde_as(schemars = true)
// )]
// #[cfg_attr(
//     not(all(feature = "abi", not(target_arch = "wasm32"))),
//     serde_as(schemars = false)
// )]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct ProtocolFeesCollected<'a> {
    #[serde_as(as = "DisplayFromStr")]
    pub fee: u128,
    #[serde_as(as = "DisplayFromStr")]
    pub surplus: u128,
    pub collector: Cow<'a, AccountIdRef>,
}

impl ProtocolFeesCollected<'_> {
    #[inline]
    pub const fn total(&self) -> Option<u128> {
        self.fee.checked_add(self.surplus)
    }
}

#[must_use = "make sure to `.emit()` this event"]
// #[cfg_attr(
//     all(feature = "abi", not(target_arch = "wasm32")),
//     serde_as(schemars = true)
// )]
// #[cfg_attr(
//     not(all(feature = "abi", not(target_arch = "wasm32"))),
//     serde_as(schemars = false)
// )]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MakerSent {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub src: u128,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub dst: u128,
}

impl MakerSent {
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.src == 0 && self.dst == 0
    }
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

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
