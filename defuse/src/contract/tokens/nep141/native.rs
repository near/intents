use defuse_core::intents::tokens::NativeWithdraw;
use defuse_near_utils::env::promise_result_checked;
use near_sdk::{Gas, Promise, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_NATIVE_WITHDRAW_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn do_native_withdraw(withdraw: NativeWithdraw) -> Promise {
        require!(
            matches!(promise_result_checked(0, 0), Ok(data) if data.is_empty()),
            "near_withdraw failed",
        );

        Promise::new(withdraw.receiver_id).transfer(withdraw.amount)
    }
}
