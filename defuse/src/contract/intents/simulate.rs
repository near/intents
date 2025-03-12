use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::Inspector,
    intents::{IntentEvent, token_diff::TokenDiff, tokens::Transfer},
    tokens::Amounts,
};
use near_sdk::{AccountIdRef, CryptoHash};

pub struct SimulateInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
    pub min_deadline: Deadline,
}

impl Default for SimulateInspector {
    fn default() -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    #[inline]
    fn on_transfer(
        &mut self,
        _sender_id: &AccountIdRef,
        _transfer: &Transfer,
        _intent_hash: CryptoHash,
    ) {
    }

    #[inline]
    fn on_token_diff(
        &mut self,
        _owner_id: &AccountIdRef,
        _token_diff: &TokenDiff,
        _fees_collected: &Amounts,
        _intent_hash: CryptoHash,
    ) {
    }

    #[inline]
    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, intent_hash: CryptoHash) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(signer_id.to_owned(), ()),
            intent_hash,
        ));
    }
}
