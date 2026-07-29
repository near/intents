use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::AccessControllable;
use near_sdk::{AccountId, PromiseOrValue, ext_contract, json_types::U128};

#[ext_contract(ext_ft_withdraw)]
pub trait FungibleTokenWithdrawer: FungibleTokenReceiver + FungibleTokenWithdrawResolver {
    /// Returns number of tokens were successfully withdrawn.
    ///
    /// Optionally can specify `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    ///
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn ft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_ft_withdraw_resolver)]
pub trait FungibleTokenWithdrawResolver {
    fn ft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        amount: U128,
        is_call: bool,
    ) -> U128;
}

#[ext_contract(ext_ft_force_withdraw)]
pub trait FungibleTokenForceWithdrawer: FungibleTokenWithdrawer + AccessControllable {
    fn ft_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<U128>;
}
