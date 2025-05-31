use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::{
        Inspector,
        event_emitter::{EmittableEvent, SimulationEvents},
    },
    intents::IntentEvent,
};
use near_sdk::{AccountIdRef, CryptoHash};

pub struct SimulateInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
    pub min_deadline: Deadline,
    pub events_handler: SimulationEvents,
}

impl Default for SimulateInspector {
    fn default() -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            events_handler: SimulationEvents::new(),
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    #[inline]
    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, intent_hash: CryptoHash) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(signer_id.to_owned(), ()),
            intent_hash,
        ));
    }

    fn on_event<E: EmittableEvent>(&mut self, event: E) {
        self.events_handler.add_event(event);
    }

    fn emit_event_eventually<E: EmittableEvent>(&mut self, event: E) {
        self.events_handler.add_event(event);
    }
}
