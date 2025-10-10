// pub enum EventMode {
//     Emit,
//     Record,
// }
//
// pub struct EventSink{
//     mode: EventMode,
//     events: Vec<defuse_core::events::DefuseEvent<'static>>
// }
//
// impl Default for EventSink {
//     fn default() -> Self {
//         Self {
//             mode: EventMode::Emit,
//             events: Vec::new(),
//         }
//     }
// }
//
// impl EventSink {
//     pub fn consume_event(&mut self, event: defuse_core::events::DefuseEvent<'_>) {
//         match self.mode {
//             EventMode::Emit => event.emit(),
//             EventMode::Record => self.events.push(event.into_static()),
//         }
//     }
//
//     pub fn record_only_mode(&mut self) {
//         self.mode = EventMode::Record;
//     }
// }
