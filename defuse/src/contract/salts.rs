use std::collections::BTreeSet;

use defuse_core::{Salt, accounts::SaltRotationEvent, events::DefuseIntentEmit};
use defuse_near_utils::UnwrapOrPanic;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{assert_one_yocto, near};

use super::{Contract, ContractExt, Role};
use crate::salts::SaltManager;

#[near]
impl SaltManager for Contract {
    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    #[payable]
    fn update_current_salt(&mut self) -> Salt {
        assert_one_yocto();

        self.salts.set_new().unwrap_or_panic();
        let current = self.salts.current();

        SaltRotationEvent {
            current,
            invalidated: BTreeSet::new(),
        }
        .emit();

        current
    }

    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    #[payable]
    fn invalidate_salts(&mut self, salts: Vec<Salt>) -> Salt {
        assert_one_yocto();

        // NOTE: omits any errors
        let invalidated = salts
            .into_iter()
            .filter(|s| self.salts.invalidate(*s).is_ok())
            .collect();

        let current = self.salts.current();

        SaltRotationEvent {
            current,
            invalidated,
        }
        .emit();

        current
    }

    #[inline]
    fn is_valid_salt(&self, salt: Salt) -> bool {
        self.salts.is_valid(salt)
    }

    #[inline]
    fn current_salt(&self) -> Salt {
        self.salts.current()
    }
}
