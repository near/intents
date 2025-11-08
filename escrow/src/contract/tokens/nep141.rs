use defuse_near_utils::UnwrapOrPanic;
use defuse_token_id::{TokenId, nep141::Nep141TokenId};
use near_contract_standards::fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, PromiseResult, env, json_types::U128,
    serde_json,
};

use crate::contract::{Contract, tokens::Token};

impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_id: TokenId = Nep141TokenId::new(env::predecessor_account_id()).into();

        match self
            .on_receive(sender_id, token_id, amount.0, &msg)
            .unwrap_or_panic()
        {
            PromiseOrValue::Promise(p) => PromiseOrValue::Promise(p),
            PromiseOrValue::Value(refund) => PromiseOrValue::Value(U128(refund)),
        }
    }
}

const FT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
const FT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

const FT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
const FT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

impl Token for Nep141TokenId {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise {
        let p = ext_ft_core::ext(self.into_contract_id())
            // TODO: are we sure we have that???
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_unused_gas_weight(unused_gas.into());
        if let Some(msg) = msg {
            p.with_static_gas(
                min_gas
                    .unwrap_or(FT_TRANSFER_CALL_GAS_DEFAULT)
                    .max(FT_TRANSFER_CALL_GAS_MIN),
            )
            .ft_transfer_call(receiver_id, U128(amount), memo, msg)
        } else {
            p.with_static_gas(
                min_gas
                    .unwrap_or(FT_TRANSFER_GAS_DEFAULT)
                    .max(FT_TRANSFER_GAS_MIN),
            )
            .ft_transfer(receiver_id, U128(amount), memo)
        }
    }

    fn resolve(result_idx: u64, amount: u128, is_call: bool) -> u128 {
        match env::promise_result(result_idx) {
            PromiseResult::Successful(value) => {
                if is_call {
                    // `ft_transfer_call` returns successfully transferred amounts
                    serde_json::from_slice::<U128>(&value)
                        .unwrap_or_default()
                        .0
                        .min(amount)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount
                } else {
                    0
                }
            }
        }
    }
}
