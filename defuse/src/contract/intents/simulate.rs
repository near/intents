use defuse_core::events::DefuseEvent;
use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::Inspector,
    intents::{IntentEvent, token_diff::TokenDeltas},
};
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, serde_json};
use std::collections::HashMap;

pub struct SimulateInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
    pub min_deadline: Deadline,
    pub balance_diff: HashMap<AccountId, TokenDeltas>,
    pub events_emitted: Vec<String>,
}

impl Default for SimulateInspector {
    fn default() -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            balance_diff: HashMap::default(),
            events_emitted: Vec::new(),
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn emit_event(&mut self, event: DefuseEvent<'_>) {
        self.events_emitted
            .push(serde_json::to_string(&event).unwrap_or_panic_display());
    }

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
}
