use defuse_near_utils::UnwrapOrPanic;
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use defuse_token_id::{TokenId, nep245::Nep245TokenId};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near, require};

use crate::{
    Error,
    contract::{
        Contract, ContractExt,
        tokens::Sender,
        utils::{ResultExt, single},
    },
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
            .ok_or(Error::WrongAsset)
            .unwrap_or_panic();

        require!(amount.0 != 0, "zero amount");

        let asset: TokenId =
            ResultExt::into_ok(Nep245TokenId::new(env::predecessor_account_id(), token_id)).into();

        let refund = self
            .on_receive(sender_id, asset, amount.0, &msg)
            .unwrap_or_panic();

        PromiseOrValue::Value(vec![U128(refund)])
    }
}

const MT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
const MT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

const MT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
const MT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

impl Sender<Nep245TokenId> for Contract {
    fn send(
        asset: Nep245TokenId,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<near_sdk::Gas>,
    ) -> near_sdk::Promise {
        let (contract_id, token_id) = asset.into_contract_id_and_mt_token_id();

        let p = ext_mt_core::ext(contract_id)
            // TODO: are we sure we have that???
            .with_attached_deposit(NearToken::from_yoctonear(1));
        if let Some(msg) = msg {
            p.with_static_gas(
                min_gas
                    .unwrap_or(MT_TRANSFER_CALL_GAS_DEFAULT)
                    .max(MT_TRANSFER_CALL_GAS_MIN),
            )
            .mt_transfer_call(
                receiver_id,
                token_id,
                U128(amount),
                None, // approval
                memo,
                msg,
            )
        } else {
            p.with_static_gas(
                min_gas
                    .unwrap_or(MT_TRANSFER_GAS_DEFAULT)
                    .max(MT_TRANSFER_GAS_MIN),
            )
            .mt_transfer(
                receiver_id,
                token_id,
                U128(amount),
                None, // approval
                memo,
            )
        }
    }
}
