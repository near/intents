use defuse_core::{
    amounts::Amounts, engine::State, intents::tokens::NotifyOnTransfer, tokens::imt::ImtTokens,
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

        self.imt_mint_with_notification(&owner_id, receiver_id, tokens, memo, notification)
            .unwrap_or_panic();

        // DefuseEvent::ImtMint(Cow::Borrowed(
        //     [IntentEvent::new(
        //         AccountEvent::new(signer_id, Cow::Borrowed(&self)),
        //         intent_hash,
        //     )]
        //     .as_slice(),
        // ))
        // .emit();

        PromiseOrValue::Value(Amounts::default())
    }
}
