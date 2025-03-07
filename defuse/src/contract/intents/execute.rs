use std::borrow::Cow;

use defuse_core::{
    accounts::AccountEvent,
    engine::Inspector,
    events::DefuseEvent,
    intents::{
        token_diff::{TokenDiff, TokenDiffEvent},
        tokens::Transfer,
        IntentEvent,
    },
    tokens::Amounts,
    Deadline,
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
