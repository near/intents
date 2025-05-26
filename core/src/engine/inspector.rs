use crate::{Deadline, events::DefuseEvent};
use impl_tools::autoimpl;
use near_sdk::{AccountIdRef, CryptoHash};

use super::event_emitter::EventEmitter;

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Deadline);

    fn on_event(&mut self, event: DefuseEvent<'_>);

    fn emit_event<E: EventEmitter>(&mut self, event: E);

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash);
}
