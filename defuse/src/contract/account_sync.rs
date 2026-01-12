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
    account_sync::AccountSyncManager,
    contract::{Contract, ContractExt, Role},
};

#[near]
impl AccountSyncManager for Contract {
    #[access_control_any(roles(Role::DAO, Role::PubKeySynchronizer))]
    #[payable]
    fn force_add_public_keys(&mut self, entries: Vec<(AccountId, Vec<PublicKey>)>) {
        assert_one_yocto();

        for (account_id, keys) in entries {
            for public_key in keys {
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
    fn force_remove_public_keys(&mut self, entries: Vec<(AccountId, Vec<PublicKey>)>) {
        assert_one_yocto();

        for (account_id, keys) in entries {
            for public_key in keys {
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
