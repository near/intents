use std::borrow::Cow;

use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::Inspector,
    events::DefuseEvent,
    intents::{
        IntentEvent,
        token_diff::{TokenDiff, TokenDiffEvent},
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
    tokens::Amounts,
};
use near_sdk::{AccountIdRef, CryptoHash};

#[derive(Debug, Default)]
pub struct ExecuteInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
}

impl Inspector for ExecuteInspector {
    #[inline]
    fn on_deadline(&mut self, _deadline: Deadline) {}

    #[inline]
    fn on_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        transfer: &Transfer,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::Transfer(
            [IntentEvent::new(
                AccountEvent::new(sender_id, Cow::Borrowed(transfer)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    #[inline]
    fn on_token_diff(
        &mut self,
        owner_id: &AccountIdRef,
        token_diff: &TokenDiff,
        fees_collected: &Amounts,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::TokenDiff(
            [IntentEvent::new(
                AccountEvent::new(
                    owner_id,
                    TokenDiffEvent {
                        diff: Cow::Borrowed(token_diff),
                        fees_collected: fees_collected.clone(),
                    },
                ),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    fn on_ft_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        ft_withdraw: &FtWithdraw,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::FtWithdraw(
            [IntentEvent::new(
                AccountEvent::new(owner_id, Cow::Borrowed(ft_withdraw)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    fn on_nft_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        nft_withdraw: &NftWithdraw,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::NftWithdraw(
            [IntentEvent::new(
                AccountEvent::new(owner_id, Cow::Borrowed(nft_withdraw)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    fn on_mt_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        mt_withdraw: &MtWithdraw,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::MtWithdraw(
            [IntentEvent::new(
                AccountEvent::new(owner_id, Cow::Borrowed(mt_withdraw)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    fn on_native_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        native_withdraw: &NativeWithdraw,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::NativeWithdraw(
            [IntentEvent::new(
                AccountEvent::new(owner_id, Cow::Borrowed(native_withdraw)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    fn on_storage_deposit(
        &mut self,
        owner_id: &AccountIdRef,
        storage_deposit: &StorageDeposit,
        intent_hash: CryptoHash,
    ) {
        DefuseEvent::StorageDeposit(
            [IntentEvent::new(
                AccountEvent::new(owner_id, Cow::Borrowed(storage_deposit)),
                intent_hash,
            )]
            .as_slice()
            .into(),
        )
        .emit();
    }

    #[inline]
    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, intent_hash: CryptoHash) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(Cow::Owned(signer_id.to_owned()), ()),
            intent_hash,
        ));
    }
}

impl Drop for ExecuteInspector {
    fn drop(&mut self) {
        if !self.intents_executed.is_empty() {
            DefuseEvent::IntentsExecuted(self.intents_executed.as_slice().into()).emit();
        }
    }
}
