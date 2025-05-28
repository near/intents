pub mod event_emitter;
mod inspector;
mod state;

use std::{borrow::Cow, ops::Deref};

pub use self::{inspector::*, state::*};

use defuse_crypto::{Payload, PublicKey, SignedPayload};
use near_sdk::{AccountId, FunctionError};

use crate::{
    DefuseError, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    events::DefuseEvent,
    fees::{FeeChangedEvent, FeeCollectorChangedEvent, Pips},
    intents::{DefuseIntents, ExecutableIntent},
    payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
};

use self::deltas::{Deltas, Transfers};

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
        self.inspector.on_intent_executed(&signer_id, hash);

        Ok(())
    }

    #[inline]
    fn finalize(self) -> Result<Transfers> {
        self.state
            .finalize()
            .map_err(DefuseError::InvariantViolated)
    }

    #[inline]
    pub fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) {
        if !self.state.add_public_key(account_id.clone(), public_key) {
            DefuseError::PublicKeyExists.panic()
        }

        self.inspector
            .on_event(DefuseEvent::PublicKeyAdded(AccountEvent::new(
                Cow::Borrowed(account_id.deref()),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&public_key),
                },
            )));
    }

    #[inline]
    pub fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) {
        if !self.state.remove_public_key(account_id.clone(), public_key) {
            DefuseError::PublicKeyNotExist.panic()
        }

        self.inspector
            .on_event(DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                Cow::Borrowed(account_id.deref()),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&public_key),
                },
            )));
    }

    #[inline]
    pub fn set_fee(&mut self, fee: Pips) {
        let old_fee = self.state.fee();
        self.state.set_fee(fee);

        self.inspector
            .on_event(DefuseEvent::FeeChanged(FeeChangedEvent {
                old_fee,
                new_fee: self.state.fee(),
            }));
    }

    #[inline]
    pub fn set_fee_collector(&mut self, fee_collector: AccountId) {
        let old_fee_collector = self.state.fee_collector().into_owned();
        self.state.set_fee_collector(fee_collector);

        self.inspector
            .on_event(DefuseEvent::FeeCollectorChanged(FeeCollectorChangedEvent {
                old_fee_collector: Cow::Borrowed(&old_fee_collector),
                new_fee_collector: self.state.fee_collector(),
            }));
    }
}
