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
use crate::message::ForwardRequest;

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
        let forward_request: ForwardRequest = msg.parse().unwrap_or_panic_display();
        PromiseOrValue::Promise(
            self.wait_for_authorization(
                &sender_id,
                &vec![Nep141TokenId::new(token.clone()).into().to_string()],
                &vec![amount],
                forward_request.salt.unwrap_or_default(),
                &msg,
            )
            .then(
                Self::ext(env::current_account_id())
                    //NOTE: forward all gas, make sure that there is enough gas to resolve transfer
                    .with_static_gas(FT_CHECK_AND_FORWARD_MIN_GAS)
                    .with_unused_gas_weight(1)
                    .check_authorization_and_forward_ft(
                        token,
                        forward_request.receiver_id,
                        amount,
                        forward_request.msg,
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
                        .ft_resolve_forward(amount),
                ),
        )
    }

    #[private]
    pub fn ft_resolve_forward(&self, amount: U128) -> U128 {
        let used = promise_result_checked_json::<U128>(0)
            // Do not refund on failed `ft_transfer_call`. A known out-of-gas attack
            // makes it impossible to distinguish whether the failure occurred in
            // `ft_transfer_call` itself or in `ft_resolve_transfer` — the resolve
            // function for the `ft_on_transfer` callback. Since `ft_resolve_transfer`
            // is responsible for managing account balances and vulnerability allows for
            // opting out from that logic we choose to lock funds on the
            // proxy account instead of refunding them.
            .unwrap_or(amount);
        U128(amount.0.saturating_sub(used.0))
    }
}
