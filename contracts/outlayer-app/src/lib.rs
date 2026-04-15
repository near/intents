#[cfg(feature = "contract")]
mod contract;
pub mod error;

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
    /// Approves a new code hash and sets the code URL atomically.
    /// Admin-only. Must attach at least 1yN.
    /// Emits [`Event::SetCode`].
    fn oa_set_code(
        &mut self,
        old_code_hash: AsHex<[u8; 32]>,
        code_hash: AsHex<[u8; 32]>,
        code_url: String,
    );

    /// Sets a new admin.
    /// Admin-only. Requires 1 yoctoNEAR. No self-transfer.
    /// Emits [`Event::TransferAdmin`].
    fn oa_transfer_admin(&mut self, new_admin_id: AccountId);

    /// Returns the current admin's account ID.
    fn oa_admin_id(&self) -> &AccountId;

    /// Returns the approved code hash
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns where the code binary can be found.
    fn oa_code_url(&self) -> String;
}

use near_sdk::ext_contract;

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "near-outlayer-app"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    SetCode {
        #[serde_as(as = "Hex")]
        hash: [u8; 32],
        url: String,
    },

    #[event_version("1.0.0")]
    TransferAdmin {
        old_admin_id: Cow<'a, AccountIdRef>,
        new_admin_id: Cow<'a, AccountIdRef>,
    },
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(serializers = [borsh])]
#[derive(Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct State {
    pub admin_id: AccountId,
    #[serde_as(as = "Hex")]
    pub code_hash: [u8; 32],
    pub code_url: String,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";

    pub fn new(admin_id: impl Into<AccountId>, code_hash: [u8; 32], code_url: String) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash,
            code_url,
        }
    }

    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
