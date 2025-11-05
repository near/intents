use defuse_core::{
    intents::tokens::FtWithdraw,
    token_id::{TokenId as CoreTokenId, nep141::Nep141TokenId},
};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{
    AccountId, Gas, PromiseOrValue, PromiseResult, env, json_types::U128, near, require, serde_json,
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

        let msg = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let receiver_id = msg.receiver_id.clone();

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

        let previous_owner_ids = vec![sender_id.clone()];
        let token_ids = vec![token_id];
        let amounts = vec![U128(amount_value)];
        let message = msg.message.clone();

        let token_account = PREDECESSOR_ACCOUNT_ID.clone();
        let resolver_receiver_id = receiver_id.clone();

        let intents_to_execute = if !msg.execute_intents.is_empty() {
            Some(&msg.execute_intents)
        } else {
            None
        };

        let notification = if !msg.message.is_empty() {
            Some(&msg.message)
        } else {
            None
        };

        match (intents_to_execute, notification, &msg.refund_if_fails) {
            (Some(intents), Some(notification), refund @ true) => {
                self.execute_intents(msg.execute_intents);
                ext_mt_receiver::ext(receiver_id.clone())
                    .mt_on_transfer(
                        sender_id.clone(),
                        previous_owner_ids,
                        token_ids,
                        amounts,
                        message,
                    )
                    .then(
                        Self::ext(CURRENT_ACCOUNT_ID.clone())
                            .with_static_gas(Self::FT_RESOLVE_DEPOSIT_GAS)
                            // do not distribute remaining gas here
                            .with_unused_gas_weight(0)
                            .ft_resolve_deposit(
                                sender_id,
                                resolver_receiver_id,
                                token_account,
                                U128(amount_value),
                            ),
                    )
                    .into()
            }
            (Some(intents), Some(notification), refund @ false) => {
                let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(msg.execute_intents);

                ext_mt_receiver::ext(receiver_id.clone())
                    .mt_on_transfer(
                        sender_id.clone(),
                        previous_owner_ids,
                        token_ids,
                        amounts,
                        message,
                    )
                    .then(
                        Self::ext(CURRENT_ACCOUNT_ID.clone())
                            .with_static_gas(Self::FT_RESOLVE_DEPOSIT_GAS)
                            // do not distribute remaining gas here
                            .with_unused_gas_weight(0)
                            .ft_resolve_deposit(
                                sender_id,
                                resolver_receiver_id,
                                token_account,
                                U128(amount_value),
                            ),
                    )
                    .into()
            }
            (Some(intents), None, refund @ true) => {
                self.execute_intents(msg.execute_intents);
                PromiseOrValue::Value(U128(0))
            }
            (Some(intents), None, refund @ false) => {
                let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(msg.execute_intents);
                PromiseOrValue::Value(U128(0))
            }
            (None, Some(notification), _) => {
                env::log_str("BLAH BLAH BLAH BLAH BLAH BLAH BLAH BLAH BLAH BLAH");
                ext_mt_receiver::ext(receiver_id.clone())
                    .with_static_gas(Self::FT_RESOLVE_DEPOSIT_GAS)
                    .mt_on_transfer(
                        sender_id.clone(),
                        previous_owner_ids,
                        token_ids,
                        amounts,
                        message,
                    )
                    .then(
                        Self::ext(CURRENT_ACCOUNT_ID.clone())
                            .with_static_gas(Self::FT_RESOLVE_DEPOSIT_GAS)
                            // do not distribute remaining gas here
                            .with_unused_gas_weight(0)
                            .ft_resolve_deposit(
                                sender_id,
                                resolver_receiver_id,
                                token_account,
                                U128(amount_value),
                            ),
                    )
                    .into()
            }
            (None, None, _) => PromiseOrValue::Value(U128(0)),
        }
    }
}

#[near]
impl Contract {
    const FT_RESOLVE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn ft_resolve_deposit(
        &mut self,
        sender_id: AccountId,
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
            PromiseResult::Failed => amount.0,
        }
        .min(amount.0);

        if requested_refund == 0 {
            return U128(0);
        }

        let token_id = CoreTokenId::Nep141(Nep141TokenId::new(token.clone()));
        let available = {
            let receiver = self.accounts.get(receiver_id.as_ref());
            receiver
                .map(|account| {
                    account
                        .as_inner_unchecked()
                        .token_balances
                        .amount_for(&token_id)
                })
                .unwrap_or(0)
        };

        let refund_amount = requested_refund.min(available);
        if refund_amount == 0 {
            return U128(0);
        }

        let withdraw = FtWithdraw {
            token,
            receiver_id: sender_id,
            amount: U128(refund_amount),
            memo: Some("refund".to_string()),
            msg: None,
            storage_deposit: None,
            min_gas: None,
        };

        match self
            .internal_ft_withdraw(receiver_id, withdraw, true)
            .unwrap_or_panic()
        {
            PromiseOrValue::Promise(promise) => {
                let _ = promise;
            }
            PromiseOrValue::Value(_) => {}
        }

        U128(0)
    }
}
