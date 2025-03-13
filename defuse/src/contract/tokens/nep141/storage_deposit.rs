use defuse_core::intents::tokens::StorageDeposit;
use near_contract_standards::storage_management::ext_storage_management;
use near_sdk::{Gas, Promise, PromiseResult, env, near, require};

use crate::contract::{Contract, ContractExt, tokens::STORAGE_DEPOSIT_GAS};

#[near]
impl Contract {
    pub(crate) const DO_STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(5);

    #[private]
    pub fn do_storage_deposit(&mut self, storage_deposit: StorageDeposit) -> Promise {
        require!(
            matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
            "near_withdraw failed",
        );

        ext_storage_management::ext(storage_deposit.contract_id)
            .with_attached_deposit(storage_deposit.amount)
            .with_static_gas(STORAGE_DEPOSIT_GAS)
            .storage_deposit(Some(storage_deposit.account_id), None)
    }
}
