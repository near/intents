use super::TokenId;

use derive_more::derive::From;
use near_account_id::AccountIdRef;
use std::borrow::Cow;

#[cfg(feature = "serde")]
use {defuse_serde_utils::cow::AsCow, serde_with::DisplayFromStr};

#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize),
    cfg_attr(
        not(feature = "near-contract"),
        must_use,
        derive(::serde::Serialize),
        serde(tag = "event", content = "data", rename_all = "snake_case")
    ),
    cfg_attr(
        feature = "near-contract",
        must_use = "make sure to `.emit()` this event",
        ::near_sdk::near(event_json(standard = "nep245"))
    )
)]
#[derive(Debug, Clone, From)]
pub enum MtEvent<'a> {
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtMint(Cow<'a, [MtMintEvent<'a>]>),
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtBurn(Cow<'a, [MtBurnEvent<'a>]>),
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    MtTransfer(Cow<'a, [MtTransferEvent<'a>]>),
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone)]
pub struct MtMintEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[cfg_attr(feature = "serde", serde_as(as = "AsCow<DisplayFromStr>"))]
    pub amounts: Cow<'a, [u128]>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone)]
pub struct MtBurnEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[cfg_attr(feature = "serde", serde_as(as = "AsCow<DisplayFromStr>"))]
    pub amounts: Cow<'a, [u128]>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone)]
pub struct MtTransferEvent<'a> {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub old_owner_id: Cow<'a, AccountIdRef>,
    pub new_owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    #[cfg_attr(feature = "serde", serde_as(as = "AsCow<DisplayFromStr>"))]
    pub amounts: Cow<'a, [u128]>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub memo: Option<Cow<'a, str>>,
}
