use core::convert::Infallible;

use defuse_near_utils::UnwrapOrPanic;
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near, require};

use crate::{
    Error,
    contract::{Contract, ContractExt, tokens::Sendable},
    token_id::{TokenId, nep245::Nep245TokenId},
};

#[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        require!(
            single(previous_owner_ids).as_ref() == Some(&sender_id),
            "approvals are not supported"
        );

        let (token_id, amount) = single(token_ids)
            .zip(single(amounts))
            .ok_or(Error::WrongToken)
            .unwrap_or_panic();

        let token_id: TokenId = Nep245TokenId::new(env::predecessor_account_id(), token_id)
            // `.into_ok()` isn't stabilized yet
            .unwrap_or_else(|err: Infallible| match err {})
            .into();

        match self
            .on_receive(sender_id, token_id, amount.0, &msg)
            .unwrap_or_panic()
        {
            PromiseOrValue::Promise(p) => PromiseOrValue::Promise(p),
            PromiseOrValue::Value(refund) => PromiseOrValue::Value(vec![U128(refund)]),
        }
    }
}

const MT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
const MT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

const MT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
const MT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

impl Sendable for Nep245TokenId {
    #[inline]
    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        if is_call {
            (MT_TRANSFER_CALL_GAS_MIN, MT_TRANSFER_CALL_GAS_DEFAULT)
        } else {
            (MT_TRANSFER_GAS_MIN, MT_TRANSFER_GAS_DEFAULT)
        }
    }

    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> near_sdk::Promise {
        let gas = self.transfer_gas(min_gas, msg.is_some());

        let (contract_id, token_id) = self.into_contract_id_and_mt_token_id();

        let p = ext_mt_core::ext(contract_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(gas)
            .with_unused_gas_weight(unused_gas.into());
        if let Some(msg) = msg {
            p.mt_transfer_call(
                receiver_id,
                token_id,
                U128(amount),
                None, // approval
                memo,
                msg,
            )
        } else {
            p.mt_transfer(
                receiver_id,
                token_id,
                U128(amount),
                None, // approval
                memo,
            )
        }
    }
}

fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}
