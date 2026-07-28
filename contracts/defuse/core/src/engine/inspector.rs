use crate::{Nonce, Timestamp, events::DefuseEvent};
use impl_tools::autoimpl;
use near_account_id::AccountIdRef;
use near_sdk::CryptoHash;

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Timestamp);

    fn on_event(&mut self, event: DefuseEvent<'_>);

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash, nonce: Nonce);
}
