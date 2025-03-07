use std::borrow::Cow;

use defuse_near_utils::{UnwrapOrPanic, UnwrapOrPanicError};
use defuse_nep245::{
    resolver::MultiTokenResolver, ClearedApproval, MtEventEmit, MtTransferEvent, TokenId,
};
use near_sdk::{env, json_types::U128, near, require, serde_json, AccountId, Gas, PromiseResult};

use crate::contract::{Contract, ContractExt};

pub(super) const MT_RESOLVE_TRANSFER_GAS: Gas = Gas::from_tgas(7);

#[near]
impl MultiTokenResolver for Contract {
    #[private]
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        #[allow(unused_mut)] mut amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128> {
        require!(approvals.is_none(), "approvals are not supported");
        require!(
            !token_ids.is_empty()
                && previous_owner_ids.len() == token_ids.len()
                && amounts.len() == token_ids.len(),
            "inavlid args"
        );

        let mut refunds = match env::promise_result(0) {
            PromiseResult::Successful(value) => serde_json::from_slice::<Vec<U128>>(&value)
                .ok()
                .filter(|refund| refund.len() == amounts.len())
                .unwrap_or_else(|| amounts.clone()),
            PromiseResult::Failed => amounts.clone(),
        };

        let sender_id = previous_owner_ids.first().cloned().unwrap_or_panic();

        for ((token_id, previous_owner_id), (amount, refund)) in token_ids
            .iter()
            .map(|token_id| token_id.parse().unwrap_or_panic_display())
            .zip(previous_owner_ids)
            .zip(amounts.iter_mut().zip(&mut refunds))
        {
            require!(
                sender_id == previous_owner_id,
                "approvals are not supported"
            );

            refund.0 = refund.0.min(amount.0);
            let Some(receiver) = self.accounts.get_mut(&receiver_id) else {
                // receiver doesn't have an account, so nowhere to refund from
                return amounts;
            };
            let receiver_balance = receiver.token_balances.amount_for(&token_id);
            // refund maximum what we can
            refund.0 = refund.0.min(receiver_balance);
            if refund.0 == 0 {
                // noting to refund
                continue;
            }

            // withdraw refund
            receiver
                .token_balances
                .sub(token_id.clone(), refund.0)
                .unwrap_or_panic();
            // deposit refund
            let previous_owner = self.accounts.get_or_create(previous_owner_id);
            previous_owner
                .token_balances
                .add(token_id, refund.0)
                .unwrap_or_panic();

            // update as used amount in-place
            amount.0 -= refund.0;
        }

        let (refunded_token_ids, refunded_amounts): (Vec<_>, Vec<_>) = token_ids
            .into_iter()
            .zip(refunds)
            .filter(|(_token_id, refund)| refund.0 > 0)
            .unzip();

        if !refunded_amounts.is_empty() {
            // deposit refunds
            Cow::Borrowed(
                [MtTransferEvent {
                    authorized_id: None,
                    old_owner_id: receiver_id.into(),
                    new_owner_id: sender_id.into(),
                    token_ids: refunded_token_ids.into(),
                    amounts: refunded_amounts.into(),
                    memo: Some("refund".into()),
                }]
                .as_slice(),
            )
            .emit();
        }

        amounts
    }
}
