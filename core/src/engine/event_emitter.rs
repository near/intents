use crate::events::DefuseEvent;

pub trait EventEmitter {
    fn do_emit(&mut self);
}

impl EventEmitter for DefuseEvent<'_> {
    fn do_emit(&mut self) {
        self.emit();
    }
}

pub struct EventHandler<E: EventEmitter> {
    postponed_events: Vec<E>,
}

impl<E: EventEmitter> EventHandler<E> {
    pub fn emit_event_owned(mut event: E) {
        event.do_emit();
    }

    pub fn emit_event_mut(event: &mut E) {
        event.do_emit();
    }

    pub fn postpone_event_emission(&mut self, event: E) {
        self.postponed_events.push(event);
    }

    fn emit_postponed(&mut self) {
        let mut events = std::mem::take(&mut self.postponed_events);
        if events.is_empty() {
            return;
        }
        events.iter_mut().for_each(|e| e.do_emit());
    }
}

impl<E: EventEmitter> Drop for EventHandler<E> {
    fn drop(&mut self) {
        self.emit_postponed();
    }
}
