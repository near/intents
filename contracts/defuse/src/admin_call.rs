use defuse_near_promise::actions::NearAction;
use near_sdk::{AccountId, Promise, ext_contract};

#[ext_contract(ext_admin_call)]
pub trait AdminCallManager {
    /// Allows the caller to execute an arbitrary function call or transfer.
    fn admin_call(&mut self, receiver_id: AccountId, action: NearAction) -> Promise;
}
