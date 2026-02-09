use std::borrow::Cow;

use defuse_core::{
    accounts::AccountEvent,
    engine::State,
    events::DefuseEvent,
    intents::{IntentEvent, imt::ImtBurn},
    tokens::imt::ImtTokens,
};

use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{AccountId, assert_one_yocto, near};

use crate::{
    contract::{Contract, ContractExt},
    tokens::imt::ImtBurner,
};

// TODO: should we return burned tokens and amounts?
#[near]
impl ImtBurner for Contract {
    #[payable]
    fn imt_burn(&mut self, minter_id: AccountId, tokens: ImtTokens, memo: Option<String>) {
        assert_one_yocto();

        let owner_id = self.ensure_auth_predecessor_id();

        self.internal_imt_burn(&minter_id, &owner_id, tokens.clone(), memo.clone())
            .unwrap_or_panic();

        DefuseEvent::ImtBurn(Cow::Borrowed(
            [IntentEvent::new(
                AccountEvent::new(
                    owner_id,
                    Cow::Owned(ImtBurn {
                        minter_id,
                        tokens,
                        memo,
                    }),
                ),
                [0; 32], // TODO: fix when MaybeIntentEvent is merged
            )]
            .as_slice(),
        ))
        .emit();
    }
}
