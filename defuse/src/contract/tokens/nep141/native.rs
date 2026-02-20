use defuse_core::intents::tokens::NativeWithdraw;
use defuse_near_utils::promise_result_checked_void;
use near_sdk::{Gas, Promise, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_NATIVE_WITHDRAW_GAS: Gas = Gas::from_tgas(12);

    #[private]
    pub fn do_native_withdraw(withdraw: NativeWithdraw) -> Promise {
        require!(
            promise_result_checked_void(0).is_ok(),
            "near_withdraw failed",
        );

        Promise::new(withdraw.receiver_id).transfer(withdraw.amount)
    }
}
