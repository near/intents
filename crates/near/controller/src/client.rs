use near_kit::contract;

use crate::UpgradeArgs;

#[contract]
pub trait ControllerUpgradable {
    /// Requires 1yN attached for security purposes
    #[call]
    #[borsh]
    fn upgrade(&mut self, args: UpgradeArgs<'_>);
}
