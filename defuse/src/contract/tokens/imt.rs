use std::borrow::Cow;

use defuse_core::{
    accounts::AccountEvent,
    engine::State,
    events::DefuseEvent,
    intents::{MaybeIntentEvent, imt::ImtBurn},
    tokens::imt::ImtTokens,
};

use defuse_near_utils::UnwrapOrPanic;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, assert_one_yocto, near};

use crate::{
    contract::{Contract, ContractExt},
    tokens::imt::ImtBurner,
};

#[near]
impl ImtBurner for Contract {
    #[pause]
    #[payable]
    fn imt_burn(&mut self, minter_id: AccountId, tokens: ImtTokens, memo: Option<String>) {
        assert_one_yocto();

        let owner_id = self.ensure_auth_predecessor_id();

        State::imt_burn(self, &owner_id, &minter_id, tokens.clone(), memo.clone())
            .unwrap_or_panic();

        DefuseEvent::ImtBurn(Cow::Borrowed(
            [MaybeIntentEvent::new(AccountEvent::new(
                owner_id,
                Cow::Owned(ImtBurn {
                    minter_id,
                    tokens,
                    memo,
                }),
            ))]
            .as_slice(),
        ))
        .emit();
    }
}
