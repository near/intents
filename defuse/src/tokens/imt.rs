#![allow(clippy::too_many_arguments)]

use defuse_core::tokens::imt::ImtTokens;
use near_sdk::{AccountId, ext_contract};

#[ext_contract(ext_imt_burn)]
pub trait ImtBurner {
    /// Burn a set of imt tokens, within the intents contract
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn imt_burn(&mut self, minter_id: AccountId, tokens: ImtTokens, memo: Option<String>);
}
