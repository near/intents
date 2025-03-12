use defuse_core::fees::Pips;
use near_plugins::AccessControllable;
use near_sdk::{AccountId, ext_contract};

#[ext_contract(ext_fees_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait FeesManager: AccessControllable {
    /// Set fees for both token_in and token_out
    fn set_fee(&mut self, fee: Pips);
    fn fee(&self) -> Pips;

    fn set_fee_collector(&mut self, fee_collector: AccountId);
    fn fee_collector(&self) -> &AccountId;
}
