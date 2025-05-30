use super::{Engine, Inspector, State, StateView};
use crate::Result;
use crate::tokens::TokenId;
use crate::{
    DefuseError,
    accounts::{AccountEvent, PublicKeyEvent},
    events::DefuseEvent,
    fees::{FeeChangedEvent, FeeCollectorChangedEvent, Pips},
};
use defuse_crypto::PublicKey;
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent, MtTransferEvent};
use near_sdk::{AccountId, AccountIdRef, FunctionError};
use std::borrow::Cow;

pub struct StateMutator<'a, S, I> {
    engine: &'a mut Engine<S, I>,
}

impl<'a, S, I> StateMutator<'a, S, I>
where
    S: State,
    I: Inspector,
{
    pub fn new(engine: &'a mut Engine<S, I>) -> Self {
        Self { engine }
    }

    #[inline]
    pub fn add_public_key(&mut self, account_id: &AccountIdRef, public_key: PublicKey) {
        if !self
            .engine
            .state
            .add_public_key(account_id.to_owned(), public_key)
        {
            DefuseError::PublicKeyExists.panic()
        }

        self.engine
            .inspector
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
            .engine
            .state
            .remove_public_key(account_id.to_owned(), public_key)
        {
            DefuseError::PublicKeyNotExist.panic()
        }

        self.engine
            .inspector
            .on_event(DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                Cow::Borrowed(account_id),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&public_key),
                },
            )));
    }

    #[inline]
    pub fn set_fee(&mut self, fee: Pips) {
        let old_fee = self.engine.state.fee();
        self.engine.state.set_fee(fee);

        self.engine
            .inspector
            .on_event(DefuseEvent::FeeChanged(FeeChangedEvent {
                old_fee,
                new_fee: self.engine.state.fee(),
            }));
    }

    #[inline]
    pub fn set_fee_collector(&mut self, fee_collector: AccountId) {
        let old_fee_collector = self.engine.state.fee_collector().into_owned();
        self.engine.state.set_fee_collector(fee_collector);

        self.engine
            .inspector
            .on_event(DefuseEvent::FeeCollectorChanged(FeeCollectorChangedEvent {
                old_fee_collector: Cow::Borrowed(&old_fee_collector),
                new_fee_collector: self.engine.state.fee_collector(),
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
        self.engine.state.internal_mt_batch_transfer(
            sender_id,
            receiver_id.clone(),
            token_ids.clone(),
            amounts.clone(),
            memo,
        )?;

        self.engine.inspector.on_event(MtEvent::MtTransfer(
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

        self.engine
            .state
            .deposit(owner_id.clone(), tokens.clone(), memo)?;

        if !tokens.is_empty() {
            let token_ids = tokens
                .iter()
                .map(|(tid, _amount)| tid.to_string())
                .collect::<Vec<_>>();

            let amounts = tokens
                .iter()
                .map(|(_tid, amount)| near_sdk::json_types::U128(*amount))
                .collect::<Vec<_>>();

            self.engine.inspector.on_event(MtEvent::MtMint(
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

        self.engine
            .state
            .withdraw(owner_id, token_amounts.clone(), memo)?;

        if !token_amounts.is_empty() {
            let token_ids = token_amounts
                .iter()
                .map(|(tid, _amount)| tid.to_string())
                .collect::<Vec<_>>();

            let amounts = token_amounts
                .iter()
                .map(|(_tid, amount)| near_sdk::json_types::U128(*amount))
                .collect::<Vec<_>>();

            self.engine
                .inspector
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
