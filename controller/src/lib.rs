use near_sdk::{Gas, Promise, ext_contract};

#[ext_contract(ext_controller_upgradable)]
pub trait ControllerUpgradable {
    /// Requires 1yN attached for security purposes
    fn upgrade(
        &mut self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] state_migration_gas: Option<Gas>,
    ) -> Promise;

    /// MUST be #[private]
    fn state_migrate(&mut self);
}
