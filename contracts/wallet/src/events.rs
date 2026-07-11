//! Events emitted by the [`Wallet`](crate::contract::Wallet) contract

use std::borrow::Cow;

use near_account_id::AccountIdRef;

/// Event emitted by the [`Wallet`](crate::contract::Wallet) contract
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
    cfg_attr(feature = "near-contract", ::near_sdk::near(event_json(standard = "wallet")))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletEvent<'a> {
    /// An extension has been added.
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    ExtensionAdded {
        /// Account id of the extension
        account_id: Cow<'a, AccountIdRef>,
        /// Actor of the corresponding request
        by: Actor<'a>,
    },

    /// An extension has been removed.
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    ExtensionRemoved {
        /// Account id of the extension
        account_id: Cow<'a, AccountIdRef>,
        /// Actor of the corresponding request
        by: Actor<'a>,
    },

    /// Signature mode mode has been set.
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    SignatureModeSet {
        /// Whether the signature has been enabled or disabled.
        enabled: bool,
        /// Actor of the corresponding request
        by: Actor<'a>,
    },

    /// Signed request has been executed.
    #[cfg_attr(feature = "near-contract", event_version("1.0.0"))]
    SignedRequest {
        /// Canonical [hash](crate::RequestMessage::hash)
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))]
        hash: [u8; 32],
    },
}

/// Actor of the request
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor<'a> {
    /// Executed by signed request with given hash via `w_execute_signed()`.
    SignedRequest(
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))] [u8; 32],
    ),

    /// Extension with given `account_id`
    Extension(Cow<'a, AccountIdRef>),
}

impl Actor<'_> {
    pub fn as_ref(&self) -> Actor<'_> {
        match self {
            Self::SignedRequest(hash) => Actor::SignedRequest(*hash),
            Self::Extension(account_id) => Actor::Extension(account_id.as_ref().into()),
        }
    }
}
