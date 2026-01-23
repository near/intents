use std::{borrow::Cow, collections::BTreeMap};

use defuse_token_id::TokenId;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    DefuseError, Result,
    accounts::AccountEvent,
    amounts::Amounts,
    engine::{Engine, Inspector, State},
    events::DefuseEvent,
    intents::tokens::NotifyOnTransfer,
};

use super::{ExecutableIntent, IntentEvent};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Mint a set of tokens from the signer to a specified account id, within the intents contract.
pub struct ImtMint {
    pub receiver_id: AccountId,

    // The tokens transferred in this call will be wrapped
    // in such a way as to bind the token ID to the minter authority.
    // The final string representation of the token
    // will be as follows: `imt:<minter_id>:<token_id>`
    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: ImtTokens,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Optionally notify receiver_id via `mt_on_transfer()`
    ///
    /// NOTE: `min_gas` is adjusted with following values:
    /// * default: 30TGas
    /// * minimum: 5TGas
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

        let tokens = self.tokens.into_generic_tokens(signer_id);

        engine
            .state
            .mt_mint(self.receiver_id.clone(), tokens.clone(), self.memo)?;

        if let Some(notification) = self.notification {
            engine
                .state
                .notify_on_transfer(signer_id, self.receiver_id, tokens, notification);
        }

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Burn a set of tokens minted by signer, within the intents contract.
pub struct ImtBurn {
    // The tokens burned in this call should be wrapped
    // as Imt tokens bounded to the minter authority
    // as follows: `imt:<minter_id>:<token_id>`
    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: Amounts,

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

        self.tokens
            .iter()
            .all(|(token, _)| matches!(token, TokenId::Imt(_)))
            .then_some(())
            .ok_or(DefuseError::OnlyImtTokensCanBeBurned)?;

        engine.state.mt_burn(signer_id, self.tokens, self.memo)
    }
}

pub type ImtTokens = Amounts<BTreeMap<defuse_nep245::TokenId, u128>>;

impl ImtTokens {
    fn into_generic_tokens(self, minter_id: &AccountIdRef) -> Amounts<BTreeMap<TokenId, u128>> {
        Amounts::new(
            self.iter()
                .map(|(token_id, amount)| {
                    let token =
                        defuse_token_id::imt::ImtTokenId::new(minter_id, token_id.to_string())
                            .into();
                    (token, *amount)
                })
                .collect(),
        )
    }
}
