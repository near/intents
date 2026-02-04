use std::borrow::Cow;

use defuse_core::{
    accounts::AccountEvent,
    amounts::Amounts,
    engine::State,
    events::{DefuseEvent, MaybeIntentEvent},
    intents::tokens::NotifyOnTransfer,
    tokens::imt::{ImtMintEvent, ImtTokens},
};

use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{AccountId, PromiseOrValue, assert_one_yocto, near};

use crate::{
    contract::{Contract, ContractExt},
    tokens::imt::ImtMinter,
};

#[near]
impl ImtMinter for Contract {
    #[payable]
    fn imt_mint(
        &mut self,
        receiver_id: AccountId,
        tokens: ImtTokens,
        memo: Option<String>,
        notification: Option<NotifyOnTransfer>,
    ) -> PromiseOrValue<Amounts> {
        assert_one_yocto();

        let owner_id = self.ensure_auth_predecessor_id();

        DefuseEvent::ImtMint(
            vec![MaybeIntentEvent::direct(AccountEvent::new(
                owner_id.clone(),
                ImtMintEvent {
                    receiver_id: Cow::Owned(receiver_id.clone()),
                    tokens: tokens.clone(),
                    memo: Cow::Owned(memo.clone()),
                },
            ))]
            .into(),
        )
        .emit();

        let minted_tokens = self
            .imt_mint_with_notification(&owner_id, receiver_id, tokens, memo, notification)
            .unwrap_or_panic();

        PromiseOrValue::Value(minted_tokens)
    }
}
