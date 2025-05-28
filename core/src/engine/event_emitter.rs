use crate::events::DefuseEvent;
use defuse_nep245::MtEvent;
use near_sdk::serde_json;

pub trait EmittableEvent {
    fn do_emit(&mut self);
    fn emit_to_json(&mut self) -> serde_json::Value;
}

impl EmittableEvent for DefuseEvent<'_> {
    fn do_emit(&mut self) {
        self.emit();
    }

    fn emit_to_json(&mut self) -> serde_json::Value {
        self.to_json()
    }
}

impl EmittableEvent for MtEvent<'_> {
    fn do_emit(&mut self) {
        self.emit();
    }

    fn emit_to_json(&mut self) -> serde_json::Value {
        self.to_json()
    }
}

#[derive(Debug, Default)]
pub struct SimulationEvents {
    events: Vec<serde_json::Value>,
}

impl SimulationEvents {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn add_event<E: EmittableEvent>(&mut self, mut event: E) {
        self.events.push(event.emit_to_json());
    }

    pub fn take(self) -> Vec<serde_json::Value> {
        self.events
    }
}
