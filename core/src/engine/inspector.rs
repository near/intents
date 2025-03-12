use impl_tools::autoimpl;
use near_sdk::{AccountIdRef, CryptoHash};

use crate::{
    Deadline,
    intents::{token_diff::TokenDiff, tokens::Transfer},
    tokens::Amounts,
};

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Deadline);

    fn on_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        transfer: &Transfer,
        intent_hash: CryptoHash,
    );
    fn on_token_diff(
        &mut self,
        owner_id: &AccountIdRef,
        token_diff: &TokenDiff,
        fees_collected: &Amounts,
        intent_hash: CryptoHash,
    );

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash);
}
