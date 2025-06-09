use defuse_core::intents::tokens::NativeWithdraw;
use near_sdk::{Gas, Promise, PromiseResult, env, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_NATIVE_WITHDRAW_GAS: Gas = Gas::from_tgas(4)
        // Transfer NEAR
        .saturating_add(Gas::from_tgas(1));

    #[private]
    pub fn do_native_withdraw(&mut self, withdraw: NativeWithdraw) -> Promise {
        require!(
            matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
            "near_withdraw failed",
        );

        Promise::new(withdraw.receiver_id).transfer(withdraw.amount)
    }
}
