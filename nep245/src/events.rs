use super::TokenId;
use derive_more::derive::From;
use near_sdk::{AccountIdRef, json_types::U128, near, serde::Deserialize};
use std::borrow::Cow;

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "nep245"))]
#[derive(Debug, Clone, Deserialize, From, PartialEq, Eq)]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    derive(schemars::JsonSchema)
)]
pub enum MtEvent<'a> {
    #[event_version("1.0.0")]
    MtMint(Cow<'a, [MtMintEvent<'a>]>),
    #[event_version("1.0.0")]
    MtBurn(Cow<'a, [MtBurnEvent<'a>]>),
    #[event_version("1.0.0")]
    MtTransfer(Cow<'a, [MtTransferEvent<'a>]>),
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MtMintEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MtBurnEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MtTransferEvent<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub old_owner_id: Cow<'a, AccountIdRef>,
    pub new_owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

impl MtEvent<'_> {
    /// Convert `MtEvent` to 'static lifetime by converting all borrowed data to owned
    pub fn into_owned(self) -> MtEvent<'static> {
        match self {
            MtEvent::MtMint(events) => {
                let owned_events: Vec<_> = events
                    .into_owned()
                    .into_iter()
                    .map(|e| MtMintEvent {
                        owner_id: Cow::Owned(e.owner_id.into_owned()),
                        token_ids: Cow::Owned(e.token_ids.into_owned()),
                        amounts: Cow::Owned(e.amounts.into_owned()),
                        memo: e.memo.map(|m| Cow::Owned(m.into_owned())),
                    })
                    .collect();
                MtEvent::MtMint(Cow::Owned(owned_events))
            }
            MtEvent::MtBurn(events) => {
                let owned_events: Vec<_> = events
                    .into_owned()
                    .into_iter()
                    .map(|e| MtBurnEvent {
                        owner_id: Cow::Owned(e.owner_id.into_owned()),
                        authorized_id: e.authorized_id.map(|a| Cow::Owned(a.into_owned())),
                        token_ids: Cow::Owned(e.token_ids.into_owned()),
                        amounts: Cow::Owned(e.amounts.into_owned()),
                        memo: e.memo.map(|m| Cow::Owned(m.into_owned())),
                    })
                    .collect();
                MtEvent::MtBurn(Cow::Owned(owned_events))
            }
            MtEvent::MtTransfer(events) => {
                let owned_events: Vec<_> = events
                    .into_owned()
                    .into_iter()
                    .map(|e| MtTransferEvent {
                        authorized_id: e.authorized_id.map(|a| Cow::Owned(a.into_owned())),
                        old_owner_id: Cow::Owned(e.old_owner_id.into_owned()),
                        new_owner_id: Cow::Owned(e.new_owner_id.into_owned()),
                        token_ids: Cow::Owned(e.token_ids.into_owned()),
                        amounts: Cow::Owned(e.amounts.into_owned()),
                        memo: e.memo.map(|m| Cow::Owned(m.into_owned())),
                    })
                    .collect();
                MtEvent::MtTransfer(Cow::Owned(owned_events))
            }
        }
    }
}
