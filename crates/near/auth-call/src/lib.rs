use near_sdk::{AccountId, PromiseOrValue, ext_contract};

#[ext_contract(ext_auth_callee)]
pub trait AuthCallee {
    /// Perform some actions on behalf of `signer_id`.
    ///
    /// Verification of `signer_id` is done by the
    /// [`predecessor_id`](::near_sdk::env::predecessor_account_id),
    /// so the implementation MUST whitelist allowed callers.
    ///
    /// NOTE: implementations are recommended to be `#[payable]`
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()>;
}
