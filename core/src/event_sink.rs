use std::collections::VecDeque;

use crate::events;

pub enum EventMode {
    Emit,
    Record,
}

pub struct EventSink {
    mode: EventMode,
    recorded_events: VecDeque<events::DefuseEvent<'static>>,
    postponed_events: Vec<events::DefuseEvent<'static>>,
}

impl Default for EventSink {
    fn default() -> Self {
        Self {
            mode: EventMode::Emit,
            recorded_events: VecDeque::new(),
            postponed_events: Vec::new(),
        }
    }
}

impl EventSink {
    pub fn postpone_event(&mut self, event: events::DefuseEvent<'static>) {
        match self.mode {
            EventMode::Emit => self.postponed_events.push(event),
            EventMode::Record => self.recorded_events.push_back(event),
        }
    }

    pub fn consume_event(&mut self, event: events::DefuseEvent<'_>) {
        match self.mode {
            EventMode::Emit => match event {
                events::DefuseEvent::Dip4Event(defuse_event) => defuse_event.emit(),
                events::DefuseEvent::Nep245Event(mt_event) => mt_event.emit(),
            },
            EventMode::Record => self.recorded_events.push_front(event.into_owned()),
        }
    }

    pub const fn record_only_mode(&mut self) {
        self.mode = EventMode::Record;
    }

    #[allow(clippy::missing_const_for_fn)] // False positive: cannot be const due to Vec deref
    pub fn recorded_events(&self) -> Vec<events::DefuseEvent<'static>> {
        self.recorded_events.iter().cloned().collect()
    }
}

impl Drop for EventSink {
    fn drop(&mut self) {
        for event in self.postponed_events.drain(..) {
            event.emit();
        }
    }
}
