use defuse_near_utils::{UnwrapOrPanicError, promise_result_U128, promise_result_bool};
use near_contract_standards::fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near};

use crate::contract::{Contract, ContractExt};
use crate::message::TransferMessage;

#[near]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // For FT, the token is identified by the predecessor (FT contract)
        let token_contract = env::predecessor_account_id();
        let token_ids = vec![token_contract.to_string()];
        let amounts = vec![amount];
        let transfer_message: TransferMessage = msg.parse().unwrap_or_panic_display();
        let cv_wait = self.wait_for_authorization(
            &sender_id,
            &token_ids,
            &amounts,
            transfer_message.salt,
            &msg,
        );

        PromiseOrValue::Promise(
            cv_wait.then(
                Self::ext(env::current_account_id())
                    .with_unused_gas_weight(1)
                    .check_authorization_and_forward_ft(
                        token_contract,
                        transfer_message.receiver_id,
                        amount,
                        transfer_message.msg,
                    ),
            ),
        )
    }
}

#[near]
impl Contract {
    #[private]
    pub fn check_authorization_and_forward_ft(
        &self,
        token_contract: AccountId,
        escrow_address: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        if !promise_result_bool(0).unwrap_or(false) {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        PromiseOrValue::Promise(
            ext_ft_core::ext(token_contract)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(Gas::from_tgas(50))
                .ft_transfer_call(
                    escrow_address,
                    amount,
                    Some("proxy forward".to_string()), // memo
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(10))
                        .resolve_ft_transfer(amount),
                ),
        )
    }

    #[private]
    pub fn resolve_ft_transfer(&self, original_amount: U128) -> U128 {
        let used = promise_result_U128(0).unwrap_or_default();
        U128(original_amount.0.saturating_sub(used.0))
    }
}
