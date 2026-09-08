use defuse_near_promise::actions::NearAction;
use near_sdk::{AccountId, Promise, ext_contract};

#[ext_contract(ext_arbitrary_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait ArbitraryManager {
    /// Allows the caller to execute an arbitrary function
    /// call or transfer on the contract.
    fn arbitrary_call(&mut self, receiver_id: AccountId, action: NearAction) -> Promise;
}
