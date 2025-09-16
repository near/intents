mod inspector;
mod state;

pub use self::{inspector::*, state::*};

use defuse_crypto::{Payload, SignedPayload};

use crate::{
    DefuseError, ExpirableNonce, Result,
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

        if ExpirableNonce::maybe_from(nonce).is_some_and(|n| deadline > n.deadline) {
            return Err(DefuseError::DeadlineGreaterThanNonce);
        }

        // make sure message is still valid
        if deadline.has_expired() {
            return Err(DefuseError::DeadlineExpired);
        }

        // make sure the account has this public key
        if !self.state.has_public_key(&signer_id, &public_key) {
            return Err(DefuseError::PublicKeyNotExist(signer_id, public_key));
        }

        // commit nonce
        self.state.commit_nonce(signer_id.clone(), nonce)?;

        intents.execute_intent(&signer_id, self, hash)?;
        self.inspector.on_intent_executed(&signer_id, hash, nonce);

        Ok(())
    }

    #[inline]
    fn finalize(self) -> Result<Transfers> {
        self.state
            .finalize()
            .map_err(DefuseError::InvariantViolated)
    }
}
