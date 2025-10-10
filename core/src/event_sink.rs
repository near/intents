use crate::events;

pub enum EventMode {
    Emit,
    Record,
}

pub struct EventSink {
    mode: EventMode,
    events: Vec<events::DefuseEvent<'static>>,
}

impl Default for EventSink {
    fn default() -> Self {
        Self {
            mode: EventMode::Emit,
            events: Vec::new(),
        }
    }
}

impl EventSink {
    pub fn consume_event(&mut self, event: events::DefuseEvent<'_>) {
        match self.mode {
            EventMode::Emit => event.emit(),
            EventMode::Record => self.events.push(event.into_static()),
        }
    }

    pub const fn record_only_mode(&mut self) {
        self.mode = EventMode::Record;
    }

    #[allow(clippy::missing_const_for_fn)] // False positive: cannot be const due to Vec deref
    pub fn recorded_events(&self) -> &[events::DefuseEvent<'static>] {
        &self.events
    }
}
