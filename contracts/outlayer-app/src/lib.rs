#[cfg(feature = "contract")]
mod contract;
pub mod error;
pub mod utils;

pub use utils::Url;

use std::{borrow::Cow, collections::BTreeMap};

pub use defuse_borsh_utils::adapters::{AsWrap, Remainder};
pub use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, borsh, near,
    serde::Serialize,
    serde_with::{hex::Hex, serde_as},
};

/// Per-app code configuration, deployed as a global contract instance per app.
#[ext_contract(ext_outlayer_app)]
pub trait OutlayerApp {
    /// Approves a new code hash.
    /// Admin-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::Approve`].
    fn op_approve(&mut self, new_hash: AsHex<[u8; 32]>);

    /// Sets a new admin.
    /// Admin-only. Requires 1 yoctoNEAR. No self-transfer.
    /// Emits [`Event::Transfer`].
    fn op_set_admin_id(&mut self, new_admin_id: AccountId);

    /// Sets the code URI. Admin-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::SetCodeUri`].
    fn op_set_code_uri(&mut self, url: Url);

    /// Returns the current admin's account ID.
    fn op_admin_id(&self) -> &AccountId;

    /// Returns the approved code hash
    /// or `0000..000` if none.
    fn op_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns where the code binary can be found.
    fn op_code_uri(&self) -> Url;
}

use near_sdk::ext_contract;

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "near-outlayer-app"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    Approve {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Transfer {
        old_admin_id: Cow<'a, AccountIdRef>,
        new_admin_id: Cow<'a, AccountIdRef>,
    },

    #[event_version("1.0.0")]
    SetCodeUri { url: Url },
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(serializers = [borsh])]
#[derive(Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct State {
    pub admin_id: AccountId,
    #[serde_as(as = "Hex")]
    pub code_hash: [u8; 32],
    pub code_url: Url,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0u8; 32];

    pub fn new(admin_id: impl Into<AccountId>, code_url: Url) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash: Self::DEFAULT_HASH,
            code_url,
        }
    }

    #[must_use]
    pub const fn pre_approve(mut self, hash: [u8; 32]) -> Self {
        self.code_hash = hash;
        self
    }

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
