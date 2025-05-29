use super::event_emitter::EmittableEvent;
use crate::Deadline;
use impl_tools::autoimpl;
use near_sdk::{AccountIdRef, CryptoHash};

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Deadline);

    fn on_event<E: EmittableEvent>(&mut self, event: E);

    fn emit_event_eventually<E: EmittableEvent + 'static>(&mut self, event: E);

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash);
}
