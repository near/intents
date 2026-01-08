use defuse_core::token_id::TokenId;
use near_sdk::{AccountId, ext_contract, json_types::U128};

#[ext_contract(ext_far_mint_manager)]
pub trait FarMint {
    /// Mints tokens to user.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn mint_tokens(&mut self, receiver_id: AccountId, token_id: TokenId, amount: U128);
}
