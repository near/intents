/// Extensions defined in this module should be used only in FAR chain
/// and are not expected to be supported on the main chain
use defuse_core::crypto::PublicKey;
use near_plugins::AccessControllable;
use near_sdk::{AccountId, ext_contract};
use std::collections::HashMap;
use std::collections::HashSet;

#[ext_contract(ext_force_public_key_manager)]
pub trait ForcePublicKeyManager: AccessControllable {
    /// Registers or re-activates `public_key` under the user account_id.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn force_add_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>);

    /// Deactivate `public_key` from the user account_id,
    /// i.e. this key can't be used to make any actions unless it's re-created.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn force_remove_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>);
}
