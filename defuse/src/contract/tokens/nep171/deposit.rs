use defuse_core::{
    intents::tokens::NftWithdraw,
    token_id::{TokenId as CoreTokenId, nep171::Nep171TokenId},
};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, Gas, Promise, PromiseOrValue, PromiseResult, env, near, require};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
};

#[near]
impl NonFungibleTokenReceiver for Contract {
    /// Deposit non-fungible token.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[pause]
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: near_contract_standards::non_fungible_token::TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        #[allow(clippy::no_effect_underscore_binding)]
        let _previous_owner_id = previous_owner_id;

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

        let core_token_id: CoreTokenId = Nep171TokenId::new(PREDECESSOR_ACCOUNT_ID.clone(), token_id.clone())
            .unwrap_or_panic_display()
            .into();

        self.deposit(receiver_id.clone(), [(core_token_id.clone(), 1)], Some("deposit"))
            .unwrap_or_panic();

        let has_message = !message.is_empty();
        let token_account = PREDECESSOR_ACCOUNT_ID.clone();
        let resolver_receiver_id = receiver_id.clone();

        let intents_promise: Option<Promise> = if execute_intents.is_empty() {
            None
        } else if refund_if_fails {
            self.execute_intents(execute_intents);
            None
        } else {
            Some(
                ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(execute_intents),
            )
        };

        if !has_message {
            return PromiseOrValue::Value(false);
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone())
            .mt_on_transfer(
                sender_id.clone(),
                vec![sender_id.clone()],
                vec![core_token_id.to_string()],
                vec![near_sdk::json_types::U128(1)],
                message,
            );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::NFT_RESOLVE_DEPOSIT_GAS)
            .with_unused_gas_weight(0)
            .nft_resolve_deposit(
                sender_id,
                resolver_receiver_id,
                token_account,
                token_id,
            );

        match intents_promise {
            Some(promise) => promise.then(notification).then(resolution).into(),
            None => notification.then(resolution).into(),
        }
    }
}

#[near]
impl Contract {
    const NFT_RESOLVE_DEPOSIT_GAS: Gas = Gas::from_tgas(50);

    #[private]
    pub fn nft_resolve_deposit(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token: AccountId,
        token_id: near_contract_standards::non_fungible_token::TokenId,
    ) -> PromiseOrValue<bool> {
        require!(
            env::predecessor_account_id() == *CURRENT_ACCOUNT_ID,
            "only self"
        );

        let should_refund = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                near_sdk::serde_json::from_slice::<Vec<near_sdk::json_types::U128>>(&value)
                    .ok()
                    .and_then(|refunds| refunds.first().cloned())
                    .map(|refund| refund.0 > 0)
                    .unwrap_or(true)
            }
            // as in token standard spec, refund on failure
            PromiseResult::Failed => true,
        };

        if !should_refund {
            return PromiseOrValue::Value(false);
        }

        let core_token_id = CoreTokenId::Nep171(
            Nep171TokenId::new(token.clone(), token_id.clone()).unwrap_or_panic_display()
        );
        let available = {
            let receiver = self.accounts.get(receiver_id.as_ref());
            receiver
                .map(|account| {
                    account
                        .as_inner_unchecked()
                        .token_balances
                        .amount_for(&core_token_id)
                })
                .unwrap_or(0)
        };

        if available == 0 {
            return PromiseOrValue::Value(false);
        }

        let withdraw = NftWithdraw {
            token,
            token_id,
            receiver_id: sender_id,
            memo: Some("refund".to_string()),
            msg: None,
            storage_deposit: None,
            min_gas: None,
        };



        // Withdraw the NFT from receiver's internal balance
        // The NFT contract will handle returning the actual NFT to the sender

        match self
            .internal_nft_withdraw(receiver_id, withdraw, true)
            .unwrap_or_panic()
        {
            PromiseOrValue::Promise(promise) => {
                let _ = promise;
            }
            PromiseOrValue::Value(_) => {}
        }

        PromiseOrValue::Value(true)
    }
}
