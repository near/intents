use std::{cell::RefCell, rc::Rc};

use crate::{
    Deadline, EventSink, Nonce,
    accounts::{AccountEvent, NonceEvent},
    events::{DefuseEvent, Dip4Event},
    intents::IntentEvent,
};
use impl_tools::autoimpl;
use near_sdk::{AccountIdRef, CryptoHash};

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Deadline);

    fn on_event(&mut self, event: Dip4Event<'_>);

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash, nonce: Nonce);
}

pub struct InspectorImpl {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, NonceEvent>>>,
    event_sink: Rc<RefCell<EventSink>>,
    pub min_deadline: Deadline,
}

impl InspectorImpl {
    pub const fn new(event_sink: Rc<RefCell<EventSink>>) -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            event_sink,
        }
    }

    pub fn get_events(&self) -> Vec<DefuseEvent<'static>> {
        self.event_sink.borrow().recorded_events()
    }
}

impl Inspector for InspectorImpl {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    fn on_event(&mut self, event: Dip4Event<'_>) {
        self.event_sink.borrow_mut().consume_event(event.into());
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
