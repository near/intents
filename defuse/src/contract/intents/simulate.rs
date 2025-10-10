use defuse_core::{
    Deadline, Nonce,
    accounts::{AccountEvent, NonceEvent},
    engine::Inspector,
    events::DefuseEvent,
    intents::IntentEvent,
};
use near_sdk::{AccountIdRef, CryptoHash};

pub struct SimulateInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, NonceEvent>>>,
    pub events: Vec<DefuseEvent<'static>>,
    pub min_deadline: Deadline,
}

impl Default for SimulateInspector {
    fn default() -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            events: Vec::new(),
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    fn on_event(&mut self, event: DefuseEvent<'_>) {
        self.events.push(event.into_static());
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
