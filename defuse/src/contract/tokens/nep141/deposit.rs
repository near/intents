use defuse_core::{
    token_id::{TokenId as CoreTokenId, nep141::Nep141TokenId},
};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{
    AccountId, Gas, Promise, PromiseOrValue, PromiseResult, env, json_types::U128, near, require,
    serde_json,
};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
};

#[near]
impl FungibleTokenReceiver for Contract {
    /// Deposit fungible tokens.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[pause]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let amount_value = amount.0;
        require!(amount_value > 0, "zero amount");

        let DepositMessage {
            receiver_id,
            execute_intents,
            refund_if_fails,
            message,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        self.deposit(
            receiver_id.clone(),
            [(
                Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone()).into(),
                amount_value,
            )],
            Some("deposit"),
        )
        .unwrap_or_panic();

        let token_id =
            CoreTokenId::Nep141(Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone())).to_string();
        let has_message = !message.is_empty();
        let token_account = PREDECESSOR_ACCOUNT_ID.clone();
        let resolver_receiver_id = receiver_id.clone();

        let intents_promise: Option<Promise> = if execute_intents.is_empty() {
            None
        } else if refund_if_fails {
            self.execute_intents(execute_intents);
            None
        } else {
            Some(ext_intents::ext(CURRENT_ACCOUNT_ID.clone()).execute_intents(execute_intents))
        };

        if !has_message {
            return PromiseOrValue::Value(U128(0));
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone()).mt_on_transfer(
            sender_id.clone(),
            vec![sender_id.clone()],
            vec![token_id],
            vec![U128(amount_value)],
            message,
        );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::FT_RESOLVE_DEPOSIT_GAS)
            .with_unused_gas_weight(0)
            .ft_resolve_deposit(
                sender_id,
                resolver_receiver_id,
                token_account,
                U128(amount_value),
            );

        match intents_promise {
            Some(promise) => promise.then(notification).then(resolution).into(),
            None => notification.then(resolution).into(),
        }
    }
}

#[near]
impl Contract {
    const FT_RESOLVE_DEPOSIT_GAS: Gas = Gas::from_tgas(50);

    #[private]
    pub fn ft_resolve_deposit(
        &mut self,
        _sender_id: AccountId,
        receiver_id: AccountId,
        token: AccountId,
        amount: U128,
    ) -> U128 {
        require!(
            env::predecessor_account_id() == *CURRENT_ACCOUNT_ID,
            "only self"
        );

        let requested_refund = match env::promise_result(0) {
            PromiseResult::Successful(value) => serde_json::from_slice::<Vec<U128>>(&value)
                .ok()
                .and_then(|refunds| refunds.first().cloned())
                .map(|refund| refund.0)
                .unwrap_or(amount.0),
            // as in token standard spec, refund whole amount in case of failure
            // PromiseResult::Failed => amount.0,
            PromiseResult::Failed => 0u128,
        }
        .min(amount.0);

        let token_id = CoreTokenId::Nep141(Nep141TokenId::new(token.clone()));

        let refunds = self.resolve_deposit_internal(
            &receiver_id,
            vec![token_id],
            vec![amount.0],
            vec![requested_refund],
        );

        U128(refunds[0])
    }
}
