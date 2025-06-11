use crate::events::DefuseEvent;
use defuse_nep245::MtEvent;
use impl_tools::autoimpl;
use near_sdk::serde_json;
use std::{rc::Rc, sync::Arc};

#[autoimpl(for <T:trait+?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait EmittableEvent: std::fmt::Debug {
    // FIXME: rename emit() - even if we use a fully-qualified name
    // FIXME: see if we can remove mut
    fn do_emit(&self);
    // FIXME: rename to_json()
    // FIXME: see if we can remove mut
    fn emit_to_json(&self) -> serde_json::Value;
}

impl EmittableEvent for DefuseEvent<'_> {
    fn do_emit(&self) {
        self.emit();
    }

    fn emit_to_json(&self) -> serde_json::Value {
        self.to_json()
    }
}

impl EmittableEvent for MtEvent<'_> {
    fn do_emit(&self) {
        self.emit();
    }

    fn emit_to_json(&self) -> serde_json::Value {
        self.to_json()
    }
}

#[derive(Debug, Default)]
pub struct SimulationEvents {
    events: Vec<serde_json::Value>,
}

impl SimulationEvents {
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn add_event<E: EmittableEvent>(&mut self, event: E) {
        self.events.push(event.emit_to_json());
    }

    pub fn take(self) -> Vec<serde_json::Value> {
        self.events
    }
}
