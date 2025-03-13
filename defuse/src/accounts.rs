use std::collections::HashSet;

use defuse_core::{Nonce, crypto::PublicKey};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, ext_contract};

#[ext_contract(ext_public_key_manager)]
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
    fn remove_public_key(&mut self, public_key: &PublicKey);

    /// Returns whether given nonce was already used by the account
    /// NOTE: nonces are non-sequential and follow
    /// [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema).
    fn is_nonce_used(&self, account_id: &AccountId, nonce: AsBase64<Nonce>) -> bool;

    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn invalidate_nonces(&mut self, nonces: Vec<AsBase64<Nonce>>);
}
