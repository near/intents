use std::collections::HashSet;

use defuse_core::{Nonce, crypto::PublicKey};
use defuse_serde_utils::base64::AsBase64;
use near_plugins::AccessControllable;
use near_sdk::{AccountId, ext_contract};

#[ext_contract(ext_account_manager)]
pub trait AccountManager {
    /// Check if account has given public key
    fn has_public_key(&self, account_id: &AccountId, public_key: &PublicKey) -> bool;

    /// Returns set of public keys registered for given account
    fn public_keys_of(&self, account_id: &AccountId) -> HashSet<PublicKey>;

    /// Registers or re-activates `public_key` under the caller account_id.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn add_public_key(&mut self, public_key: PublicKey);

    /// Deactivate `public_key` from the caller account_id,
    /// i.e. this key can't be used to make any actions unless it's re-created.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn remove_public_key(&mut self, public_key: PublicKey);

    /// Returns whether given nonce was already used by the account
    /// NOTE: nonces are non-sequential and follow
    /// [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema).
    fn is_nonce_used(&self, account_id: &AccountId, nonce: AsBase64<Nonce>) -> bool;

    /// Returns whether authentication by PREDECESSOR_ID is enabled
    /// for given `account_id`.
    ///
    /// NOTE: Authentication by PREDECESSOR_ID is enabled by default
    /// when creating new accounts.
    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountId) -> bool;

    /// Disables authentication by PREDECESSOR_ID for the caller,
    /// i.e. PREDECESSOR_ID itself.
    ///
    /// **WARN**: Doing so might lock you out of your funds if
    /// you don't have any other public_keys added to your account.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn disable_auth_by_predecessor_id(&mut self);
}

#[ext_contract(ext_force_account_locker)]
pub trait AccountForceLocker: AccessControllable {
    /// Returns whether the given`account_id` is locked
    fn is_account_locked(&self, account_id: &AccountId) -> bool;

    /// Locks given `account_id` from modifying its own state, including
    /// token balances.
    /// Returns `false` if the account was already in locked state.
    ///
    /// Attached deposit of 1yN is required for security purposes.
    ///
    /// NOTE: this still allows for force withdrawals/transfers
    fn force_lock_account(&mut self, account_id: AccountId) -> bool;

    /// Unlocks given `account_id`.
    /// Returns `false` if the account wasn't in locked state.
    ///
    /// Attached deposit of 1yN is required for security purposes.
    fn force_unlock_account(&mut self, account_id: &AccountId) -> bool;
}
