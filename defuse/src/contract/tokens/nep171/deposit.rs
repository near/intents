use defuse_core::token_id::{TokenId as CoreTokenId, nep171::Nep171TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, Gas, Promise, PromiseOrValue, near};

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

        let core_token_id: CoreTokenId = Nep171TokenId::new(PREDECESSOR_ACCOUNT_ID.clone(), token_id)
            .unwrap_or_panic_display()
            .into();

        self.deposit(receiver_id.clone(), [(core_token_id.clone(), 1)], Some("deposit"))
            .unwrap_or_panic();

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

        if message.is_empty() {
            return PromiseOrValue::Value(false);
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone())
            .mt_on_transfer(
                sender_id.clone(),
                vec![sender_id],
                vec![core_token_id.to_string()],
                vec![near_sdk::json_types::U128(1)],
                message,
            );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::NFT_RESOLVE_DEPOSIT_GAS)
            .with_unused_gas_weight(0)
            .nft_resolve_deposit(&receiver_id, vec![core_token_id], vec![1]);

        match intents_promise {
            Some(promise) => promise.then(notification).then(resolution).into(),
            None => notification.then(resolution).into(),
        }
    }
}

#[near]
impl Contract {
    //TODO: figure out prcise value
    const NFT_RESOLVE_DEPOSIT_GAS: Gas = Gas::from_tgas(50);

    #[private]
    pub fn nft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<CoreTokenId>,
        deposited_amounts: Vec<u128>,
    ) -> PromiseOrValue<bool> {
        self.resolve_deposit_internal(receiver_id, token_ids, deposited_amounts)
            .first()
            .copied()
            .map(|elem| PromiseOrValue::Value( elem == 1.into()))
            .unwrap()
    }
}
