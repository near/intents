use std::collections::HashSet;

use near_sdk::{ext_contract, serde::Serialize, AccountId};

use crate::{crypto::PublicKey, nep413::U256, utils::serde::wrappers::Base64};

use super::Result;

#[ext_contract(ext_public_key_manager)]
pub trait AccountManager {
    /// Check if account has given public key
    fn has_public_key(&self, account_id: &AccountId, public_key: &PublicKey) -> bool;

    /// Returns set of public keys registered for given account
    fn public_keys_of(&self, account_id: &AccountId) -> HashSet<PublicKey>;

    /// Registers or re-activates `public_key` under the caller account_id.
    fn add_public_key(&mut self, public_key: PublicKey);

    /// Deactivate `public_key` from the caller account_id,
    /// i.e. this key can't be used to make any actions unless it's re-created.
    fn remove_public_key(&mut self, public_key: &PublicKey);

    /// Returns whether given nonce was already used by the account
    /// NOTE: nonces are non-sequential and follow
    /// [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema).
    fn is_nonce_used(&self, account_id: &AccountId, nonce: Base64<U256>) -> bool;

    #[handle_result]
    fn invalidate_nonces(&mut self, nonces: Vec<Base64<U256>>) -> Result<()>;
}

#[must_use = "make sure to `.emit()` this event"]
#[derive(Debug, Serialize)]
#[serde(crate = "::near_sdk::serde")]
pub struct PublicKeyAddedEvent<'a> {
    pub account_id: &'a AccountId,
    pub public_key: &'a PublicKey,
}

#[must_use = "make sure to `.emit()` this event"]
#[derive(Debug, Serialize)]
#[serde(crate = "::near_sdk::serde")]
pub struct PublicKeyRemovedEvent<'a> {
    pub account_id: &'a AccountId,
    pub public_key: &'a PublicKey,
}
