use std::borrow::Cow;

use near_account_id::AccountIdRef;

/// An event emitted by [`OutlayerApp`](crate::contract::OutlayerApp) contract.
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    cfg_attr(
        // `#[near(event_json(...)))]` does it for us
        not(feature = "near-contract"),
        derive(::serde::Serialize),
        serde(tag = "event", content = "data")
    ),
    cfg_attr(feature = "near-contract", ::near_sdk::near(event_json(standard = "outlayer-app")))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutlayerAppEvent<'a> {
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    SetCode {
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
        hash: [u8; 32],
        url: Cow<'a, str>,
    },

    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    TransferAdmin {
        old_admin_id: Cow<'a, AccountIdRef>,
        new_admin_id: Cow<'a, AccountIdRef>,
    },
}
