pub mod accounts;
pub mod amounts;
mod deadline;
pub mod engine;
mod error;
pub mod events;
pub mod fees;
pub mod intents;
mod nonce;
pub mod payload;

pub use self::{deadline::*, error::*, nonce::*};

pub use defuse_crypto as crypto;
pub use defuse_erc191 as erc191;
pub use defuse_nep413 as nep413;
pub use defuse_sep53 as sep53;
pub use defuse_tip191 as tip191;
pub use defuse_token_id as token_id;
pub use defuse_ton_connect as ton_connect;

pub enum EventMode {
    Emit,
    Record,
}

pub struct EventSink{
    mode: EventMode,
    events: Vec<events::DefuseEvent<'static>>
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

    pub fn record_only_mode(&mut self) {
        self.mode = EventMode::Record;
    }

    pub fn recorded_events(&self) -> &[events::DefuseEvent<'static>] {
        &self.events
    }
} 

