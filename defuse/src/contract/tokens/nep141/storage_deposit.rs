use defuse_core::intents::tokens::StorageDeposit;
use defuse_near_utils::promise_result_checked_void;
use near_contract_standards::storage_management::ext_storage_management;
use near_sdk::{Gas, Promise, near, require};

use crate::contract::{Contract, ContractExt, tokens::STORAGE_DEPOSIT_GAS};

#[near]
impl Contract {
    pub(crate) const DO_STORAGE_DEPOSIT_GAS: Gas =
        Gas::from_tgas(5).saturating_add(STORAGE_DEPOSIT_GAS);

    #[private]
    pub fn do_storage_deposit(storage_deposit: StorageDeposit) -> Promise {
        require!(
            promise_result_checked_void(0).is_ok(),
            "near_withdraw failed",
        );

        ext_storage_management::ext(storage_deposit.contract_id)
            .with_attached_deposit(storage_deposit.amount)
            .with_static_gas(STORAGE_DEPOSIT_GAS)
            // do not distribute remaining gas here
            .with_unused_gas_weight(0)
            .storage_deposit(Some(storage_deposit.deposit_for_account_id), None)
    }
}
