#![allow(clippy::too_many_arguments)]

use defuse_core::{amounts::Amounts, intents::tokens::NotifyOnTransfer, tokens::imt::ImtTokens};
use near_sdk::{AccountId, PromiseOrValue, ext_contract};

#[ext_contract(ext_imt_mint)]
pub trait ImtMinter {
    /// Returns tokens and amounts which were successfully minted
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn imt_mint(
        &mut self,
        receiver_id: AccountId,
        tokens: ImtTokens,
        memo: Option<String>,
        notification: Option<NotifyOnTransfer>,
    ) -> PromiseOrValue<Amounts>;
}
