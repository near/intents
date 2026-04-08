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

/// Per-app code configuration, deployed as a global contract instance per app.
#[ext_contract(ext_outlayer_app)]
pub trait OutlayerApp {
    /// Approves a new code hash.
    /// Admin-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::Approve`].
    fn op_approve(&mut self, new_hash: AsHex<[u8; 32]>);

    /// Uploads the code binary. Admin-only.
    /// Requires `sha256(code)` to match `code_hash`.
    /// Sets `location` to `OnChain { account: current_account_id, storage_prefix: CODE_PREFIX }`.
    /// Emits [`Event::Upload`].
    fn op_upload_code(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>);

    /// Sets a new admin.
    /// Admin-only. Requires 1 yoctoNEAR. No self-transfer.
    /// Emits [`Event::Transfer`].
    fn op_set_admin_id(&mut self, new_admin_id: AccountId);

    /// Sets the code location. Admin-only. Requires 1 yoctoNEAR.
    /// Emits [`Event::SetLocation`].
    fn op_set_location(&mut self, location: CodeLocation);

    /// Returns the current admin's account ID.
    fn op_admin_id(&self) -> &AccountId;

    /// Returns the approved code hash
    /// or `0000..000` if none.
    fn op_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the uploaded code bytes, or `None` if not yet uploaded.
    fn op_code(&self) -> Option<AsBase64<Vec<u8>>>;

    /// Returns where the code binary can be found, or `None` if not set.
    fn op_location(&self) -> Option<CodeLocation>;
}

use near_sdk::ext_contract;

/// Describes where the code binary can be fetched from.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeLocation {
    /// Code is stored on-chain at `account`; fetch via `op_code()` view call
    /// using `storage_prefix` as the storage key.
    OnChain {
        account: AccountId,
        #[serde_as(as = "defuse_serde_utils::base64::Base64")]
        storage_prefix: Vec<u8>,
    },
    /// Code is available at this HTTP(S) URL.
    HttpUrl { url: String },
}

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
    Upload {
        #[serde_as(as = "Hex")]
        code_hash: [u8; 32],
    },

    #[event_version("1.0.0")]
    Transfer {
        old_admin_id: Cow<'a, AccountIdRef>,
        new_admin_id: Cow<'a, AccountIdRef>,
    },

    #[event_version("1.0.0")]
    SetLocation { location: CodeLocation },
}

#[serde_as(crate = "near_sdk::serde_with")]
#[near(serializers = [borsh])]
#[derive(Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct State {
    pub admin_id: AccountId,
    #[serde_as(as = "Hex")]
    pub code_hash: [u8; 32],
    #[serde(skip_serializing)]
    pub code: LazyOption<Vec<u8>>,
    /// TODO:maybe it should be set of `CodeLocation` to have backup ?
    pub location: Option<CodeLocation>,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";
    pub const DEFAULT_HASH: [u8; 32] = [0u8; 32];
    pub const CODE_PREFIX: &[u8] = b"code";

    pub fn new(admin_id: impl Into<AccountId>) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash: Self::DEFAULT_HASH,
            code: LazyOption::new(Self::CODE_PREFIX, None),
            location: None,
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
