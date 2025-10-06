use defuse_core::Salt;
use near_sdk::ext_contract;

#[ext_contract(ext_salt_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait SaltManager {
    /// Sets the current salt to a new one, previous salt remains valid.
    /// Returns the new current salt.
    fn rotate_salt(&mut self, invalidate_current: bool) -> Salt;

    /// Invalidates the provided salt: invalidates provided salt,
    /// sets a new one if it was current salt.
    /// Returns the current salt.
    fn invalidate_salt(&mut self, salt: Salt) -> Salt;

    /// Returns whether the provided salt is valid
    fn is_valid_salt(&self, salt: Salt) -> bool;

    /// Returns the current salt
    fn get_current_salt(&self) -> Salt;
}
