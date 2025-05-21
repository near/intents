use std::borrow::Cow;

use defuse_core::{
    Deadline, accounts::AccountEvent, engine::Inspector, events::DefuseEvent, intents::IntentEvent,
};
use near_sdk::{AccountIdRef, CryptoHash};

#[derive(Debug, Default)]
pub struct ExecuteInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
}

impl Inspector for ExecuteInspector {
    #[inline]
    fn on_deadline(&mut self, _deadline: Deadline) {}

    fn on_event(&mut self, event: DefuseEvent<'_>) {
        event.emit();
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
