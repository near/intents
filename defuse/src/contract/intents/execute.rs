use std::borrow::Cow;

use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::{Inspector, event_emitter::EmittableEvent},
    events::DefuseEvent,
    intents::IntentEvent,
};
use near_sdk::{AccountIdRef, CryptoHash};

#[derive(Debug, Default)]
pub struct ExecuteInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
    pub postponed_events: Vec<Box<dyn EmittableEvent>>,
}

impl Inspector for ExecuteInspector {
    #[inline]
    fn on_deadline(&mut self, _deadline: Deadline) {}

    #[inline]
    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, intent_hash: CryptoHash) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(Cow::Owned(signer_id.to_owned()), ()),
            intent_hash,
        ));
    }

    fn on_event<E: EmittableEvent>(&mut self, mut event: E) {
        event.do_emit();
    }

    fn emit_event_eventually<E: EmittableEvent + 'static>(&mut self, event: E) {
        self.postponed_events.push(Box::new(event));
    }
}

impl Drop for ExecuteInspector {
    fn drop(&mut self) {
        if !self.intents_executed.is_empty() {
            DefuseEvent::IntentsExecuted(self.intents_executed.as_slice().into()).emit();
        }

        let events = std::mem::take(&mut self.postponed_events);
        for mut event in events {
            event.do_emit();
        }
    }
}
