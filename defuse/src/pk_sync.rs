use defuse_core::crypto::PublicKey;
use near_sdk::{AccountId, ext_contract};

// TODO: locked accounts + accounts with predecessor id disabled should be also synchronized
#[ext_contract(ext_pk_sync_manager)]
pub trait PkSyncManager {
    /// Registers or re-activates `public_key` under the user account_id.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn add_user_public_keys(&mut self, public_keys: Vec<(AccountId, Vec<PublicKey>)>);

    /// Deactivate `public_key` from the user account_id,
    /// i.e. this key can't be used to make any actions unless it's re-created.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn remove_user_public_keys(&mut self, public_keys: Vec<(AccountId, Vec<PublicKey>)>);
}
