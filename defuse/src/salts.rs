use defuse_core::Salt;
use near_sdk::ext_contract;

#[ext_contract(ext_salt_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait SaltManager {
    /// Sets the current salt to a new one, previous salt remains valid
    fn rotate_salt(&mut self);

    /// Sets the current salt to a new one, previous salt is invalidated
    fn reset_salt(&mut self);

    /// Returns whether the provided salt is valid
    fn is_valid_salt(&self, salt: &Salt) -> bool;

    /// Returns the current salt
    fn get_current_salt(&self) -> Salt;
}
