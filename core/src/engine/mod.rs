pub mod event_emitter;
mod inspector;
mod state;

pub use self::{inspector::*, state::*};
use crate::{
    DefuseError, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    events::DefuseEvent,
    fees::{FeeChangedEvent, FeeCollectorChangedEvent, Pips},
    intents::{DefuseIntents, ExecutableIntent, IntentEvent},
    payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
    tokens::TokenId,
};
use defuse_crypto::{Payload, PublicKey, SignedPayload};
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent, MtTransferEvent};
use near_sdk::{AccountId, AccountIdRef, FunctionError};
use std::borrow::Cow;

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

    #[inline]
    pub fn add_public_key(&mut self, account_id: &AccountIdRef, public_key: PublicKey) {
        if !self.state.add_public_key(account_id.to_owned(), public_key) {
            DefuseError::PublicKeyExists.panic()
        }

        self.inspector
            .on_event(DefuseEvent::PublicKeyAdded(AccountEvent::new(
                Cow::Borrowed(account_id),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&public_key),
                },
            )));
    }

    #[inline]
    pub fn remove_public_key(&mut self, account_id: &AccountIdRef, public_key: PublicKey) {
        if !self
            .state
            .remove_public_key(account_id.to_owned(), public_key)
        {
            DefuseError::PublicKeyNotExist.panic()
        }

        self.inspector
            .on_event(DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                Cow::Borrowed(account_id),
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

    #[inline]
    pub fn internal_mt_batch_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<near_sdk::json_types::U128>,
        memo: Option<&str>,
    ) -> Result<()> {
        self.state.internal_mt_batch_transfer(
            sender_id,
            receiver_id.clone(),
            token_ids.clone(),
            amounts.clone(),
            memo,
        )?;

        self.inspector.on_event(MtEvent::MtTransfer(
            [MtTransferEvent {
                authorized_id: None,
                old_owner_id: sender_id.into(),
                new_owner_id: receiver_id.into(),
                token_ids: token_ids.into(),
                amounts: amounts.into(),
                memo: memo.map(Into::into),
            }]
            .as_slice()
            .into(),
        ));

        Ok(())
    }

    #[inline]
    pub fn deposit(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        let tokens = tokens.into_iter().collect::<Vec<_>>();

        self.state.deposit(owner_id.clone(), tokens.clone(), memo)?;

        if !tokens.is_empty() {
            let token_ids = tokens
                .iter()
                .map(|(tid, _amount)| tid.to_string())
                .collect::<Vec<_>>();

            let amounts = tokens
                .iter()
                .map(|(_tid, amount)| near_sdk::json_types::U128(*amount))
                .collect::<Vec<_>>();

            self.inspector.on_event(MtEvent::MtMint(
                [MtMintEvent {
                    owner_id: owner_id.into(),
                    token_ids: token_ids.into(),
                    amounts: amounts.into(),
                    memo: memo.map(Into::into),
                }]
                .as_slice()
                .into(),
            ));
        }

        Ok(())
    }

    #[inline]
    pub fn withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        let token_amounts = token_amounts.into_iter().collect::<Vec<_>>();

        self.state.withdraw(owner_id, token_amounts.clone(), memo)?;

        if !token_amounts.is_empty() {
            let token_ids = token_amounts
                .iter()
                .map(|(tid, _amount)| tid.to_string())
                .collect::<Vec<_>>();

            let amounts = token_amounts
                .iter()
                .map(|(_tid, amount)| near_sdk::json_types::U128(*amount))
                .collect::<Vec<_>>();

            self.inspector
                .emit_event_eventually(MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
                    owner_id: owner_id.to_owned().into(),
                    authorized_id: None,
                    token_ids: token_ids.into(),
                    amounts: amounts.into(),
                    memo: memo.map(|v| Cow::Owned(v.to_owned())),
                }])));
        }

        Ok(())
    }
}
