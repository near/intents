use std::borrow::Cow;

use near_sdk::{AccountIdRef, near, serde::Deserialize};

#[near(event_json(standard = "wallet"))]
#[derive(Debug, Clone, Deserialize)]
pub enum WalletEvent<'a> {
    #[event_version("1.0.0")]
    SignatureModeSet { enable: bool },
    #[event_version("1.0.0")]
    ExtensionAdded { account_id: Cow<'a, AccountIdRef> },
    #[event_version("1.0.0")]
    ExtensionRemoved { account_id: Cow<'a, AccountIdRef> },
    // TODO: query_id?
}
