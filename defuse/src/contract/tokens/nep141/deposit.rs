use defuse_core::token_id::{TokenId as CoreTokenId, nep141::Nep141TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, require};

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

        if !msg.execute_intents.is_empty() {
            if msg.refund_if_fails {
                self.execute_intents(msg.execute_intents);
            } else {
                // detach promise
                let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(msg.execute_intents);
            }
        }

        // ext_mt_receiver::ext(receiver_id)
        //     .mt_on_transfer(sender_id, previous_owner_ids, token_ids, amounts, message)
        //     .then(
        //         // schedule storage_deposit() only after near_withdraw() returns
        //         Self::ext(CURRENT_ACCOUNT_ID.clone())
        //             .with_static_gas(
        //                 Self::DO_NFT_WITHDRAW_GAS
        //                     .checked_add(withdraw.min_gas())
        //                     .ok_or(DefuseError::GasOverflow)
        //                     .unwrap_or_panic(),
        //             )
        //             .do_nft_withdraw(withdraw.clone()),
        //     ).into()

        // promise.and(ext_intents::ext(CURRENT_ACCOUNT_ID).execute_intents(msg.execute_intents));
        //
        //     .then(
        //         ext_ft_resolver::ext(env::current_account_id())
        //             .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
        //             // do not distribute remaining gas for `ft_resolve_transfer`
        //             .with_unused_gas_weight(0)
        //             .ft_resolve_transfer(sender_id, receiver_id, amount.into()),
        //     )
        //

        PromiseOrValue::Value(U128(0))
    }
}
