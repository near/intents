pub mod event_emitter;
mod inspector;
pub mod mutator;
mod state;

use self::deltas::{Deltas, Transfers};
pub use self::{inspector::*, state::*};
use crate::{
    DefuseError, Result,
    accounts::AccountEvent,
    events::DefuseEvent,
    intents::{DefuseIntents, ExecutableIntent, IntentEvent},
    payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
};
use defuse_crypto::{Payload, SignedPayload};
use mutator::StateMutator;
use std::borrow::Cow;

pub struct Engine<S, I> {
    pub state: Deltas<S>,
    pub inspector: I,
}

impl<S, I> Engine<S, I>
where
    S: State,
    I: Inspector,
{
    #[inline]
    pub fn new(state: S, inspector: I) -> Self {
        Self {
            state: Deltas::new(state),
            inspector,
        }
    }

    pub fn execute_signed_intents(
        mut self,
        signed: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<Transfers> {
        for signed in signed {
            self.execute_signed_intent(signed)?;
        }
        self.finalize()
    }

    fn execute_signed_intent(&mut self, signed: MultiPayload) -> Result<()> {
        // verify signed payload and get public key
        let public_key = signed.verify().ok_or(DefuseError::InvalidSignature)?;

        // calculate intent hash
        let hash = signed.hash();

        // extract NEP-413 payload
        let DefusePayload::<DefuseIntents> {
            signer_id,
            verifying_contract,
            deadline,
            nonce,
            message: intents,
        } = signed.extract_defuse_payload()?;

        // check recipient
        if verifying_contract != *self.state.verifying_contract() {
            return Err(DefuseError::WrongVerifyingContract);
        }

        self.inspector.on_deadline(deadline);
        // make sure message is still valid
        if deadline.has_expired() {
            return Err(DefuseError::DeadlineExpired);
        }

        // make sure the account has this public key
        if !self.state.has_public_key(&signer_id, &public_key) {
            return Err(DefuseError::PublicKeyNotExist);
        }

        // commit nonce
        if !self.state.commit_nonce(signer_id.clone(), nonce) {
            return Err(DefuseError::NonceUsed);
        }

        intents.execute_intent(&signer_id, self, hash)?;

        // Emit related events. Due to backwards compatibility, we have two functions for this,
        // and are displayed in simulations in two different ways.
        self.inspector.on_intent_executed(&signer_id, hash);
        self.inspector
            .emit_event_eventually(DefuseEvent::IntentsExecuted(Cow::Owned(vec![
                IntentEvent::new(AccountEvent::new(Cow::Owned(signer_id.clone()), ()), hash),
            ])));

        Ok(())
    }

    #[inline]
    fn finalize(self) -> Result<Transfers> {
        self.state
            .finalize()
            .map_err(DefuseError::InvariantViolated)
    }

    pub fn state_mutator(&mut self) -> StateMutator<S, I> {
        StateMutator::new(self)
    }
}
