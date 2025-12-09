use std::borrow::Cow;

use near_sdk::{AccountId, PromiseOrValue};

use crate::{
    Error, Params, Result, State,
    event::{EscrowIntentEmit, FundedEvent},
};

impl State {
    pub(super) fn fund(
        &mut self,
        params: Params,
        sender_id: AccountId,
        amount: u128,
    ) -> Result<PromiseOrValue<u128>> {
        if sender_id != params.maker {
            return Err(Error::Unauthorized);
        }

        self.maker_src_remaining = self
            .maker_src_remaining
            .checked_add(amount)
            .ok_or(Error::IntegerOverflow)?;

        FundedEvent {
            params: Cow::Owned(params),
            maker_src_added: amount,
            maker_src_remaining: self.maker_src_remaining,
        }
        .emit();

        Ok(PromiseOrValue::Value(0))
    }
}
