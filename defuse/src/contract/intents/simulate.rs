use defuse_core::events::DefuseEvent;
use defuse_core::{
    Deadline,
    accounts::AccountEvent,
    engine::Inspector,
    intents::{
        IntentEvent,
        token_diff::TokenDeltas,
        tokens::{FtWithdraw, MtWithdraw, NftWithdraw},
    },
};
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, serde_json};
use std::collections::HashMap;

pub struct SimulateInspector {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, ()>>>,
    pub min_deadline: Deadline,
    pub balance_diff: HashMap<AccountId, TokenDeltas>,
    #[allow(dead_code)] // FIXME: remove
    pub wnear_id: AccountId,
    pub ft_withdrawals: Option<Vec<FtWithdraw>>,
    pub nft_withdrawals: Option<Vec<NftWithdraw>>,
    pub mt_withdrawals: Option<Vec<MtWithdraw>>,
    pub events_emitted: Vec<String>,
}

impl SimulateInspector {
    pub fn new(wnear_id: AccountId) -> Self {
        Self {
            intents_executed: Vec::new(),
            min_deadline: Deadline::MAX,
            balance_diff: HashMap::default(),
            wnear_id,
            ft_withdrawals: None,
            nft_withdrawals: None,
            mt_withdrawals: None,
            events_emitted: Vec::new(),
        }
    }
}

impl Inspector for SimulateInspector {
    #[inline]
    fn emit_event(&mut self, event: DefuseEvent<'_>) {
        self.events_emitted
            .push(serde_json::to_string(&event).unwrap_or_panic_display());
    }

    #[inline]
    fn on_deadline(&mut self, deadline: Deadline) {
        self.min_deadline = self.min_deadline.min(deadline);
    }

    // #[inline]
    // fn on_transfer(
    //     &mut self,
    //     sender_id: &AccountIdRef,
    //     transfer: &Transfer,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     for (token_id, transfer_amount) in &transfer.tokens {
    //         self.balance_diff
    //             .entry_or_default(sender_id.to_owned())
    //             .sub(token_id.clone(), *transfer_amount)
    //             .ok_or(DefuseError::BalanceOverflow)?;

    //         self.balance_diff
    //             .entry_or_default(transfer.receiver_id.clone())
    //             .add(token_id.clone(), *transfer_amount)
    //             .ok_or(DefuseError::BalanceOverflow)?;
    //     }

    //     Ok(())
    // }

    // #[inline]
    // fn on_token_diff(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     token_diff: &TokenDiff,
    //     _fees_collected: &Amounts,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     for (token_id, delta) in &token_diff.diff {
    //         if *delta >= 0 {
    //             self.balance_diff
    //                 .entry_or_default(owner_id.to_owned())
    //                 .add(
    //                     token_id.clone(),
    //                     (*delta).try_into().unwrap_or_panic_display(),
    //                 )
    //                 .ok_or(DefuseError::BalanceOverflow)?;
    //         } else {
    //             self.balance_diff
    //                 .entry_or_default(owner_id.to_owned())
    //                 .sub(
    //                     token_id.clone(),
    //                     (-delta).try_into().unwrap_or_panic_display(),
    //                 )
    //                 .ok_or(DefuseError::BalanceOverflow)?;
    //         }
    //     }

    //     Ok(())
    // }

    // fn on_ft_withdraw(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     ft_withdraw: &FtWithdraw,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     self.balance_diff
    //         .entry_or_default(owner_id.to_owned())
    //         .sub(
    //             TokenId::Nep141(ft_withdraw.token.clone()),
    //             ft_withdraw.amount.0,
    //         )
    //         .ok_or(DefuseError::BalanceOverflow)?;

    //     self.ft_withdrawals
    //         .get_or_insert(Vec::new())
    //         .push(ft_withdraw.clone());

    //     Ok(())
    // }

    // fn on_nft_withdraw(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     nft_withdraw: &NftWithdraw,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     self.balance_diff
    //         .entry_or_default(owner_id.to_owned())
    //         .sub(
    //             TokenId::Nep171(nft_withdraw.token.clone(), nft_withdraw.token_id.clone()),
    //             1,
    //         )
    //         .ok_or(DefuseError::BalanceOverflow)?;

    //     self.nft_withdrawals
    //         .get_or_insert(Vec::new())
    //         .push(nft_withdraw.clone());

    //     Ok(())
    // }

    // fn on_mt_withdraw(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     mt_withdraw: &MtWithdraw,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     require!(
    //         mt_withdraw.amounts.len() != mt_withdraw.token_ids.len(),
    //         "Invalid mt_withdraw() call. List of tokens and amounts don't match in length."
    //     );

    //     for (token_id, transfer_amount) in
    //         mt_withdraw.token_ids.iter().zip(mt_withdraw.amounts.iter())
    //     {
    //         let token_id: TokenId = token_id.parse().unwrap_or_panic_display();

    //         self.balance_diff
    //             .entry_or_default(owner_id.to_owned())
    //             .sub(token_id, transfer_amount.0)
    //             .ok_or(DefuseError::BalanceOverflow)?;
    //     }

    //     self.mt_withdrawals
    //         .get_or_insert(Vec::new())
    //         .push(mt_withdraw.clone());

    //     Ok(())
    // }

    // fn on_native_withdraw(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     native_withdraw: &NativeWithdraw,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     let wnear = TokenId::Nep141(self.wnear_id.clone());
    //     self.balance_diff
    //         .entry_or_default(owner_id.to_owned())
    //         .sub(wnear, native_withdraw.amount.as_yoctonear())
    //         .ok_or(DefuseError::BalanceOverflow)?;

    //     Ok(())
    // }

    // fn on_storage_deposit(
    //     &mut self,
    //     owner_id: &AccountIdRef,
    //     storage_deposit: &StorageDeposit,
    //     _intent_hash: CryptoHash,
    // ) -> Result<()> {
    //     let wnear = TokenId::Nep141(self.wnear_id.clone());
    //     self.balance_diff
    //         .entry_or_default(owner_id.to_owned())
    //         .sub(wnear, storage_deposit.amount.as_yoctonear())
    //         .ok_or(DefuseError::BalanceOverflow)?;

    //     Ok(())
    // }

    #[inline]
    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, intent_hash: CryptoHash) {
        self.intents_executed.push(IntentEvent::new(
            AccountEvent::new(signer_id.to_owned(), ()),
            intent_hash,
        ));
    }
}
