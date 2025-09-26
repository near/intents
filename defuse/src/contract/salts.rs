use defuse_core::{DefuseError, Salt, accounts::SaltRotationEvent, events::DefuseIntentEmit};
use defuse_near_utils::UnwrapOrPanic;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{assert_one_yocto, near};

use super::{Contract, ContractExt, Role};
use crate::salts::SaltManager;

#[near]
impl SaltManager for Contract {
    #[payable]
    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    fn rotate_salt(&mut self) {
        assert_one_yocto();

        let old_salt = self.salts.set_new();

        SaltRotationEvent {
            new_salt: *self.salts.current(),
            old_salt,
        }
        .emit();
    }

    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    fn reset_salt(&mut self) {
        assert_one_yocto();

        let old_salt = self.salts.set_new();
        self.salts
            .clear_previous(&old_salt)
            .then_some(())
            .ok_or(DefuseError::InvalidSalt)
            .unwrap_or_panic();

        SaltRotationEvent {
            new_salt: *self.salts.current(),
            old_salt,
        }
        .emit();
    }

    #[inline]
    fn is_valid_salt(&self, salt: &Salt) -> bool {
        self.salts.is_valid(salt)
    }

    #[inline]
    fn get_current_salt(&self) -> Salt {
        *self.salts.current()
    }
}
