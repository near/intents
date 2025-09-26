use defuse_core::{Salt, ValidSalts};
use near_sdk::ext_contract;

#[ext_contract(ext_salt_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait SaltManager {
    /// Rotate the current salt to a new one
    fn rotate_salt(&mut self, salt: Salt);

    /// Reset the set of valid salts
    fn reset_salts(&mut self, salts: ValidSalts);

    fn get_valid_salts(&self) -> &ValidSalts;
}
