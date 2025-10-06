use defuse_core::Salt;
use near_sdk::ext_contract;

#[ext_contract(ext_salt_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait SaltManager {
    /// Sets the current salt to a new one, previous salt remains valid.
    /// Returns the new current salt.
    fn update_current_salt(&mut self) -> Salt;

    /// Invalidates the provided salt: invalidates provided salt,
    /// sets a new one if it was current salt.
    /// Returns the current salt.
    fn invalidate_salts(&mut self, salts: Vec<Salt>) -> Salt;

    /// Returns whether the provided salt is valid
    fn is_valid_salt(&self, salt: Salt) -> bool;

    /// Returns the current salt
    fn current_salt(&self) -> Salt;
}
