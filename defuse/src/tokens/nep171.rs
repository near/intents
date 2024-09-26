use defuse_contracts::{
    defuse::{
        tokens::{
            nep171::{NonFungibleTokenWithdrawResolver, NonFungibleTokenWithdrawer},
            TokenId,
        },
        DefuseError, Result,
    },
    utils::cache::{CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID},
};
use near_contract_standards::non_fungible_token::{
    self,
    core::{ext_nft_core, NonFungibleTokenReceiver},
};
use near_sdk::{
    assert_one_yocto, env, near, serde_json, AccountId, NearToken, PromiseOrValue, PromiseResult,
};

use crate::{DefuseImpl, DefuseImplExt};

#[near]
impl NonFungibleTokenReceiver for DefuseImpl {
    /// Deposit non-fungible token.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[allow(unused_variables)]
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: near_contract_standards::non_fungible_token::TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        let deposit_to = if !msg.is_empty() {
            msg.parse().unwrap()
        } else {
            sender_id
        };

        self.accounts
            .get_or_create(deposit_to)
            .token_balances
            .deposit(TokenId::Nep171(PREDECESSOR_ACCOUNT_ID.clone(), token_id), 1)
            .unwrap();

        PromiseOrValue::Value(false)
    }
}

#[near]
impl NonFungibleTokenWithdrawer for DefuseImpl {
    #[payable]
    fn nft_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_id: non_fungible_token::TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        self.internal_nft_withdraw(token, sender_id, token_id, memo, msg)
            .unwrap()
    }
}

impl DefuseImpl {
    fn internal_nft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        token_id: non_fungible_token::TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<PromiseOrValue<bool>> {
        let account = self
            .accounts
            .get_mut(&PREDECESSOR_ACCOUNT_ID)
            .ok_or(DefuseError::AccountNotFound)?;
        account
            .token_balances
            .withdraw(&TokenId::Nep171(token.clone(), token_id.clone()), 1)?;

        let ext =
            ext_nft_core::ext(token.clone()).with_attached_deposit(NearToken::from_yoctonear(1));
        let is_call = msg.is_some();
        Ok(if let Some(msg) = msg {
            ext.nft_transfer_call(receiver_id, token_id.clone(), None, memo, msg)
        } else {
            ext.nft_transfer(receiver_id, token_id.clone(), None, memo)
        }
        .then(
            Self::ext(CURRENT_ACCOUNT_ID.clone())
                // TODO: with static gas
                .nft_resolve_withdraw(token, PREDECESSOR_ACCOUNT_ID.clone(), token_id, is_call),
        )
        .into())
    }
}

#[near]
impl NonFungibleTokenWithdrawResolver for DefuseImpl {
    #[private]
    fn nft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_id: non_fungible_token::TokenId,
        is_call: bool,
    ) -> bool {
        let used = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                if is_call {
                    // `nft_transfer_call` returns true if token was successfully transferred
                    serde_json::from_slice(&value).unwrap_or_default()
                } else {
                    // `nft_transfer` returns empty result on success
                    value.is_empty()
                }
            }
            PromiseResult::Failed => false,
        };
        if !used {
            let account = self.accounts.get_or_create(sender_id);
            // Are we sure that we want to ignore that?
            let _ = account
                .token_balances
                .deposit(TokenId::Nep171(token, token_id), 1);
        }
        used
    }
}
