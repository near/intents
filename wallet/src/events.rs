use std::borrow::Cow;

use defuse_serde_utils::base58::Base58;
use near_sdk::{AccountIdRef, CryptoHash, near, serde::Deserialize, serde_with::serde_as};

#[serde_as(crate = "::near_sdk::serde_with")]
#[near(event_json(standard = "wallet"))]
#[derive(Debug, Clone, Deserialize)]
pub enum WalletEvent<'a> {
    #[event_version("1.0.0")]
    ExtensionAdded { account_id: Cow<'a, AccountIdRef> },

    #[event_version("1.0.0")]
    ExtensionRemoved { account_id: Cow<'a, AccountIdRef> },

    #[event_version("1.0.0")]
    SignatureModeSet { enable: bool },

    #[event_version("1.0.0")]
    SignedRequest {
        #[serde_as(as = "Base58")]
        hash: CryptoHash,
    },
}
