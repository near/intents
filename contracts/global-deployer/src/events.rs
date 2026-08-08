use std::borrow::Cow;

use near_account_id::AccountIdRef;

/// An event emitted by [`GlobalDeployer`](crate::contract::GlobalDeployer) contract.
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
        serde(tag = "event", content = "data", rename_all = "snake_case"),

    ),
    cfg_attr(feature = "near-contract", ::near_sdk::near(event_json(standard = "global-deployer")))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GdEvent<'a> {
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    Approve {
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
        code_hash: [u8; 32],
        reason: ApprovalReason<'a>,
    },

    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    Deploy {
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
        code_hash: [u8; 32],
    },

    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    Transfer {
        old_owner_id: Cow<'a, AccountIdRef>,
        new_owner_id: Cow<'a, AccountIdRef>,
    },
}

/// Reason for [`GdEvent::Approve`]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalReason<'a> {
    /// Approval was consumed by deploy
    Deploy(#[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))] [u8; 32]),

    /// Approved by owner ID
    By(Cow<'a, AccountIdRef>),
}
