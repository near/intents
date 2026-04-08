#[cfg(feature = "contract")]
mod contract;
pub mod error;

use std::{borrow::Cow, collections::BTreeMap};

pub use defuse_borsh_utils::adapters::{AsWrap, Remainder};
pub use defuse_serde_utils::base64::AsBase64;
pub use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, borsh, near,
    serde::Serialize,
    serde_with::{hex::Hex, serde_as},
    store::LazyOption,
};

/// Per-project WASM configuration, deployed as a global contract instance per project.
#[ext_contract(ext_outlayer_project)]
pub trait OutlayerProject {
    /// Approves a new WASM hash.
    /// Updater-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::Approve`].
    fn op_approve(&mut self, new_hash: AsHex<[u8; 32]>);

    /// Uploads the WASM binary. Updater-only.
    /// Requires `sha256(wasm)` to match `wasm_hash`.
    /// Sets `location` to `OnChain { account: current_account_id, storage_prefix: WASM_PREFIX }`.
    /// Emits [`Event::Upload`].
    fn op_upload_wasm(&mut self, #[serializer(borsh)] wasm: AsWrap<Vec<u8>, Remainder>);

    /// Sets a new updater. Resets `wasm_hash` to zeros.
    /// Updater-only. Requires 1 yoctoNEAR. No self-transfer.
    /// Emits [`Event::Transfer`] and [`Event::Approve`].
    fn op_set_updater_id(&mut self, new_updater_id: AccountId);

    /// Sets the WASM location. Updater-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::SetLocation`].
    fn op_set_location(&mut self, location: WasmLocation);

    /// Returns the current updater's account ID.
    fn op_updater_id(&self) -> &AccountId;

    /// Returns the approved WASM has
    /// or `0000..000` if none.
    fn op_wasm_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the uploaded WASM bytes, or `None` if not yet uploaded.
    fn op_wasm(&self) -> Option<AsBase64<Vec<u8>>>;

    /// Returns where the WASM binary can be found, or `None` if not set.
    fn op_location(&self) -> Option<WasmLocation>;
}

use near_sdk::ext_contract;

/// Describes where the WASM binary can be fetched from.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmLocation {
    /// WASM is stored on-chain at `account`; fetch via `op_wasm()` view call
    /// using `storage_prefix` as the storage key.
    OnChain {
        account: AccountId,
        #[serde_as(as = "defuse_serde_utils::base64::Base64")]
        storage_prefix: Vec<u8>,
    },
    /// WASM is available at this HTTP(S) URL.
    HttpUrl { url: String },
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "near-outlayer-project"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    #[event_version("1.0.0")]
    Approve {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Upload {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Transfer {
        old_updater_id: Cow<'a, AccountIdRef>,
        new_updater_id: Cow<'a, AccountIdRef>,
    },

    #[event_version("1.0.0")]
    SetLocation { location: WasmLocation },
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(serializers = [borsh])]
#[derive(Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct State {
    pub updater_id: AccountId,
    #[serde_as(as = "Hex")]
    pub wasm_hash: [u8; 32],
    #[serde(skip_serializing)]
    pub wasm: LazyOption<Vec<u8>>,
    /// TODO:maybe it should be set of `WasmLocation` to have backup ?
    pub location: Option<WasmLocation>,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0u8; 32];
    pub const WASM_PREFIX: &[u8] = b"wasm";

    pub fn new(updater_id: impl Into<AccountId>) -> Self {
        Self {
            updater_id: updater_id.into(),
            wasm_hash: Self::DEFAULT_HASH,
            wasm: LazyOption::new(Self::WASM_PREFIX, None),
            location: None,
        }
    }

    #[must_use]
    pub const fn pre_approve(mut self, hash: [u8; 32]) -> Self {
        self.wasm_hash = hash;
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
