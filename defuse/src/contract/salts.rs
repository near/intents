use defuse_core::{Salt, ValidSalts, accounts::SaltRotationEvent, events::DefuseIntentEmit};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{assert_one_yocto, near};

use super::{Contract, ContractExt, Role};
use crate::salts::SaltManager;

#[near]
impl SaltManager for Contract {
    // TODO: find difference
    #[pause]
    #[pause(name = "intents")]
    #[payable]
    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    fn rotate_salt(&mut self, #[allow(unused_mut)] mut salt: Salt) {
        assert_one_yocto();

        let old_salts = self.salts.rotate_salt(&mut salt).expect("invalid salt");

        SaltRotationEvent {
            old_salts,
            new_salts: self.salts.clone(),
        }
        .emit();
    }

    #[access_control_any(roles(Role::DAO, Role::SaltManager))]
    fn reset_salts(&mut self, salts: ValidSalts) {
        assert_one_yocto();

        let old_salts = self.salts.reset_salts(salts);

        SaltRotationEvent {
            old_salts,
            new_salts: self.salts.clone(),
        }
        .emit();
    }

    fn get_valid_salts(&self) -> &ValidSalts {
        &self.salts
    }
}
