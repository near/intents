use defuse_near_promise::NearPromise;
use near_sdk::ext_contract;

#[ext_contract(ext_admin_call)]
pub trait AdminCallManager {
    /// Allows the caller to execute an arbitrary promise on behalf of this contract.
    fn admin_call(&mut self, promises: Vec<NearPromise>);
}
