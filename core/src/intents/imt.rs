use near_sdk::{AccountId, AccountIdRef, CryptoHash, near};
use serde_with::{DisplayFromStr, serde_as};
use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    Result,
    accounts::AccountEvent,
    amounts::Amounts,
    engine::{Engine, Inspector, State},
    events::DefuseEvent,
    intents::{ExecutableIntent, IntentEvent, tokens::NotifyOnTransfer},
    tokens::imt::ImtTokens,
};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Mint a set of tokens from the signer to a specified account id, within the intents contract.
pub struct ImtMint {
    /// Receiver of the minted tokens
    pub receiver_id: AccountId,

    /// The token_ids will be wrapped to bind the token ID to the
    /// minter authority (i.e. signer of this intent).
    /// The final string representation of the token will be as follows:
    /// `imt:<minter_id>:<token_id>`
    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: ImtTokens,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Optionally notify receiver_id via `mt_on_transfer()`
    ///
    /// NOTE: `min_gas` is adjusted with following values:
    /// * minimum: 5TGas
    /// * default: 30TGas
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub notification: Option<NotifyOnTransfer>,
}

impl ExecutableIntent for ImtMint {
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
            .inspector
            .on_event(DefuseEvent::ImtMint(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(signer_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.imt_mint_with_notification(
            signer_id,
            self.receiver_id,
            self.tokens,
            self.memo,
            self.notification,
        )?;

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Burn a set of imt tokens, within the intents contract.
pub struct ImtBurn {
    // The minter authority of the imt tokens
    pub minter_id: AccountId,

    /// The token_ids will be wrapped to bind the token ID to the
    /// minter authority. The final string representation of the
    /// token will be as follows:
    /// `imt:<minter_id>:<token_id>`
    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: ImtTokens,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

impl ExecutableIntent for ImtBurn {
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
            .inspector
            .on_event(DefuseEvent::ImtBurn(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(signer_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine
            .state
            .imt_burn(&self.minter_id, signer_id, self.tokens, self.memo)
    }
}
