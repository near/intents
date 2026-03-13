use near_contract_standards::non_fungible_token::{TokenId, core::NonFungibleTokenReceiver};
use near_plugins::AccessControllable;
use near_sdk::{AccountId, PromiseOrValue, ext_contract};

#[ext_contract(ext_nft_withdraw)]
pub trait NonFungibleTokenWithdrawer:
    NonFungibleTokenReceiver + NonFungibleTokenWithdrawResolver
{
    /// Returns number of tokens were successfully withdrawn
    ///
    /// Optionally can specify `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn nft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<bool>;
}

#[ext_contract(ext_nft_withdraw_resolver)]
pub trait NonFungibleTokenWithdrawResolver {
    fn nft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_id: TokenId,
        is_call: bool,
    ) -> bool;
}

#[ext_contract(ext_nft_force_withdraw)]
pub trait NonFungibleTokenForceWithdrawer: NonFungibleTokenWithdrawer + AccessControllable {
    fn nft_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<bool>;
}
