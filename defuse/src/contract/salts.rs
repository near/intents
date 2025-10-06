use defuse_core::{
    DefuseError, Salt,
    accounts::{InvalidateSaltEvent, RotateSaltEvent},
    events::DefuseIntentEmit,
};
use defuse_near_utils::UnwrapOrPanic;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{assert_one_yocto, near};

use super::{Contract, ContractExt, Role};
use crate::salts::SaltManager;

#[near]
impl SaltManager for Contract {
    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    #[payable]
    fn rotate_salt(&mut self, invalidate_current: bool) -> Salt {
        assert_one_yocto();

        let old_salt = self.salts.set_new(invalidate_current);
        let current_salt = self.salts.current();

        RotateSaltEvent {
            new_salt: current_salt,
            old_salt,
        }
        .emit();

        current_salt
    }

    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    #[payable]
    fn invalidate_salt(&mut self, salt: Salt) -> Salt {
        assert_one_yocto();

        self.salts
            .invalidate(salt)
            .then_some(())
            .ok_or(DefuseError::InvalidSalt)
            .unwrap_or_panic();

        let current_salt = self.salts.current();

        InvalidateSaltEvent {
            current: current_salt,
            invalidated: salt,
        }
        .emit();

        current_salt
    }

    #[inline]
    fn is_valid_salt(&self, salt: Salt) -> bool {
        self.salts.is_valid(salt)
    }

    #[inline]
    fn get_current_salt(&self) -> Salt {
        self.salts.current()
    }
}
