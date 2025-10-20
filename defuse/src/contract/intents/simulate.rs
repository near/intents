use std::borrow::Cow;

use defuse_core::{
    Deadline, Nonce,
    accounts::{AccountEvent, NonceEvent},
    engine::Inspector,
    events::DefuseEvent,
    intents::IntentEvent,
};
use near_sdk::{AccountIdRef, CryptoHash, serde_json::Value as JsonValue};

use crate::simulation_output::SimulationReport;

pub struct SimulateInspector {
    intents_executed: Vec<IntentEvent<AccountEvent<'static, NonceEvent>>>,
    recorded_events: Vec<JsonValue>,
    min_deadline: Deadline,
}

impl SimulateInspector {
    pub fn into_report(self) -> SimulationReport {
        let intents_executed_event =
            DefuseEvent::IntentsExecuted(Cow::Borrowed(&self.intents_executed));

        SimulationReport {
            logs: self
                .recorded_events
                .into_iter()
                .chain(std::iter::once(intents_executed_event.to_json()))
                .map(|elem|
                //NOTE: match exact format of events as when emitted by near-sdk
                ::std::format!("EVENT_JSON:{elem}"))
                .collect(),
            intents_executed: self.intents_executed,
            min_deadline: self.min_deadline,
        }
    }
}

impl Default for SimulateInspector {
    fn default() -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            recorded_events: Vec::new(),
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    fn on_event(&mut self, event: DefuseEvent<'_>) {
        self.recorded_events.push(event.to_json());
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
