use near_sdk::{Gas, Promise, PublicKey, ext_contract};

pub const ADD_FULL_ACCESS_KEY_GAS: Gas = Gas::from_tgas(10);

#[ext_contract(ext_full_access_keys)]
pub trait FullAccessKeys {
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise;
    fn delete_key(&mut self, public_key: PublicKey) -> Promise;
}
