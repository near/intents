use std::borrow::Cow;

use defuse_crypto::PublicKey;
use near_sdk::{AccountIdRef, CryptoHash, near};

use crate::{
    Result,
    accounts::{AccountEvent, PublicKeyEvent},
    engine::{Engine, Inspector, State},
    events::{DefuseEvent, MaybeIntentEvent},
};

use super::ExecutableIntent;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Given an account id, the user can add public keys. The added public keys can sign
/// intents on behalf of these accounts, even to add new ones.
/// Warning: Implicit account ids, by default, have their corresponding public keys added.
/// Meaning: For a leaked private key, whose implicit account id had been used in intents,
/// the user must manually rotate the underlying public key within intents, too.
pub struct AddPublicKey {
    pub public_key: PublicKey,
}

impl ExecutableIntent for AddPublicKey {
    #[inline]
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .state
            .add_public_key(signer_id.to_owned(), self.public_key)?;

        engine
            .inspector
            .on_event(DefuseEvent::PublicKeyAdded(MaybeIntentEvent::intent(
                AccountEvent::new(
                    Cow::Borrowed(signer_id),
                    PublicKeyEvent {
                        public_key: Cow::Borrowed(&self.public_key),
                    },
                ),
                intent_hash,
            )));

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Remove the public key associated with a given account. See `AddPublicKey`.
pub struct RemovePublicKey {
    pub public_key: PublicKey,
}

impl ExecutableIntent for RemovePublicKey {
    #[inline]
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> crate::Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .state
            .remove_public_key(signer_id.to_owned(), self.public_key)?;

        engine
            .inspector
            .on_event(DefuseEvent::PublicKeyRemoved(MaybeIntentEvent::intent(
                AccountEvent::new(
                    Cow::Borrowed(signer_id),
                    PublicKeyEvent {
                        public_key: Cow::Borrowed(&self.public_key),
                    },
                ),
                intent_hash,
            )));

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct SetAuthByPredecessorId {
    pub enabled: bool,
}

impl ExecutableIntent for SetAuthByPredecessorId {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .state
            .set_auth_by_predecessor_id(signer_id.to_owned(), self.enabled)?;

        engine
            .inspector
            .on_event(DefuseEvent::SetAuthByPredecessorId(
                MaybeIntentEvent::intent(
                    AccountEvent::new(Cow::Borrowed(signer_id), Cow::Borrowed(&self)),
                    intent_hash,
                ),
            ));

        Ok(())
    }
}
