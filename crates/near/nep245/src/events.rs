use super::TokenId;
use defuse_serde_utils::cow::AsCow;
use derive_more::derive::From;
use near_account_id::AccountIdRef;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::borrow::Cow;

#[cfg_attr(
    not(feature = "near-contract"),
    derive(::serde::Serialize),
    serde(tag = "event", content = "data", rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "near-contract",
    must_use = "make sure to `.emit()` this event",
    ::near_sdk::near(event_json(standard = "nep245"))
)]
#[derive(Debug, Clone, Deserialize, From)]
pub enum MtEvent<'a> {
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtMint(Cow<'a, [MtMintEvent<'a>]>),
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtBurn(Cow<'a, [MtBurnEvent<'a>]>),
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtTransfer(Cow<'a, [MtTransferEvent<'a>]>),
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtMintEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[serde_as(as = "AsCow<DisplayFromStr>")]
    pub amounts: Cow<'a, [u128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtBurnEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[serde_as(as = "AsCow<DisplayFromStr>")]
    pub amounts: Cow<'a, [u128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtTransferEvent<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub old_owner_id: Cow<'a, AccountIdRef>,
    pub new_owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[serde_as(as = "AsCow<DisplayFromStr>")]
    pub amounts: Cow<'a, [u128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}
