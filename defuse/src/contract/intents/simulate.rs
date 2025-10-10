use std::{cell::RefCell, rc::Rc};

use defuse_core::{
    accounts::{AccountEvent, NonceEvent}, engine::Inspector, events::DefuseEvent, intents::IntentEvent, Deadline, EventSink, Nonce
};
use near_sdk::{AccountIdRef, CryptoHash};

pub struct GeneralInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, NonceEvent>>>,
    event_sink: Rc<RefCell<EventSink>>,
    pub min_deadline: Deadline,
}

impl GeneralInspector {
    pub fn new(event_sink: Rc<RefCell<EventSink>>) -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            event_sink: event_sink,
        }
    }

    pub fn get_events(&self) -> Vec<DefuseEvent<'static>> {
        self.event_sink.borrow().recorded_events().into_iter().cloned().collect()
    }
}

impl Inspector for GeneralInspector {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    fn on_event(&mut self, event: DefuseEvent<'_>) {
        self.event_sink.borrow_mut().consume_event(event.into_static());
    }

    #[inline]
    fn on_intent_executed(
        &mut self,
        signer_id: &AccountIdRef,
        intent_hash: CryptoHash,
        nonce: Nonce,
    ) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(signer_id.to_owned(), NonceEvent::new(nonce)),
            intent_hash,
        ));
    }
}
