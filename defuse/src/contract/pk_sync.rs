use std::borrow::Cow;

use defuse_core::{
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    engine::State,
    events::DefuseEvent,
};

use defuse_near_utils::UnwrapOrPanic;

use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, assert_one_yocto, near};

use crate::{
    contract::{Contract, ContractExt, Role},
    pk_sync::PkSyncManager,
};

#[near]
impl PkSyncManager for Contract {
    #[access_control_any(roles(Role::DAO, Role::PubKeySynchronizer))]
    #[payable]
    fn add_user_public_keys(&mut self, public_keys: Vec<(AccountId, Vec<PublicKey>)>) {
        assert_one_yocto();

        for (account_id, public_keys) in public_keys {
            for public_key in public_keys {
                State::add_public_key(self, account_id.clone(), public_key).unwrap_or_panic();

                DefuseEvent::PublicKeyAdded(AccountEvent::new(
                    Cow::Borrowed(account_id.as_ref()),
                    PublicKeyEvent {
                        public_key: Cow::Borrowed(&public_key),
                    },
                ))
                .emit();
            }
        }
    }

    #[access_control_any(roles(Role::DAO, Role::PubKeySynchronizer))]
    #[payable]
    fn remove_user_public_keys(&mut self, public_keys: Vec<(AccountId, Vec<PublicKey>)>) {
        assert_one_yocto();

        for (account_id, public_keys) in public_keys {
            for public_key in public_keys {
                State::remove_public_key(self, account_id.clone(), public_key).unwrap_or_panic();

                DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                    Cow::Borrowed(account_id.as_ref()),
                    PublicKeyEvent {
                        public_key: Cow::Borrowed(&public_key),
                    },
                ))
                .emit();
            }
        }
    }
}
