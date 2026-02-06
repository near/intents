use defuse_near_utils::{UnwrapOrPanicError, promise_result_checked_json};
use defuse_token_id::TokenId;
use defuse_token_id::nep141::Nep141TokenId;
use near_contract_standards::fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near};

const FT_RESOLVE_FORWARD_GAS: Gas = Gas::from_tgas(5);
const FT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);
const FT_CHECK_AND_FORWARD_MIN_GAS: Gas = Gas::from_tgas(5)
    .saturating_add(FT_TRANSFER_CALL_MIN_GAS)
    .saturating_add(FT_RESOLVE_FORWARD_GAS);

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
        let token = env::predecessor_account_id();
        let token_id = TokenId::from(Nep141TokenId::new(token.clone()));
        let token_ids = vec![token_id.to_string()];
        let amounts = vec![amount];
        let transfer_message: TransferMessage = msg.parse().unwrap_or_panic_display();
        PromiseOrValue::Promise(
            self.wait_for_authorization(
                &sender_id,
                &token_ids,
                &amounts,
                transfer_message.salt.unwrap_or_default(),
                &msg,
            )
            .then(
                Self::ext(env::current_account_id())
                    //NOTE: forward all gas, make sure that there is enough gas to resolve transfer
                    .with_static_gas(FT_CHECK_AND_FORWARD_MIN_GAS)
                    .with_unused_gas_weight(1)
                    .check_authorization_and_forward_ft(
                        token,
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
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        if !promise_result_checked_json::<bool>(0).unwrap_or(false) {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        PromiseOrValue::Promise(
            ext_ft_core::ext(token)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(FT_TRANSFER_CALL_MIN_GAS)
                .with_unused_gas_weight(1)
                .ft_transfer_call(
                    receiver_id,
                    amount,
                    Some(super::PROXY_MEMO.to_string()),
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(FT_RESOLVE_FORWARD_GAS)
                        .with_unused_gas_weight(0)
                        .resolve_ft_transfer(amount),
                ),
        )
    }

    #[private]
    pub fn resolve_ft_transfer(&self, original_amount: U128) -> U128 {
        let used = promise_result_checked_json::<U128>(0).unwrap_or(original_amount);
        U128(original_amount.0.saturating_sub(used.0))
    }
}
