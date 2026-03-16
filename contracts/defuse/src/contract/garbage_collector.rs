use defuse_core::{ExpirableNonce, Nonce, SaltedNonce, VersionedNonce, engine::State};
use defuse_serde_utils::base64::AsBase64;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, assert_one_yocto, near};

use super::{Contract, ContractExt, Role};
use crate::{garbage_collector::GarbageCollector, salts::SaltManager};

#[near]
impl GarbageCollector for Contract {
    #[access_control_any(roles(Role::DAO, Role::GarbageCollector))]
    #[payable]
    fn cleanup_nonces(&mut self, nonces: Vec<(AccountId, Vec<AsBase64<Nonce>>)>) {
        assert_one_yocto();

        for (account_id, nonces) in nonces {
            for nonce in nonces.into_iter().map(AsBase64::into_inner) {
                if !self.is_nonce_cleanable(nonce) {
                    continue;
                }

                // NOTE: all errors are omitted
                let [prefix @ .., _] = nonce;
                let _ = State::cleanup_nonce_by_prefix(self, &account_id, prefix);
            }
        }
    }
}

impl Contract {
    #[inline]
    fn is_nonce_cleanable(&self, nonce: Nonce) -> bool {
        let Some(versioned_nonce) = VersionedNonce::maybe_from(nonce) else {
            return false;
        };

        match versioned_nonce {
            VersionedNonce::V1(SaltedNonce {
                salt,
                nonce: ExpirableNonce { deadline, .. },
            }) => deadline.has_expired() || !self.is_valid_salt(salt),
        }
    }
}
