use anyhow::Result;
use near_kit::{AccountId, AccountIdRef, Gas, SigningNear, rpc::CheckedOutcome};

use crate::extensions::FnCallTransaction;

#[near_kit::contract]
pub trait AccessControllable {
    fn acl_has_role(&self, role: &str, account_id: &AccountIdRef) -> bool;

    /// Returns [`Option<bool>`]
    #[call]
    fn acl_grant_role(&mut self, role: &str, account_id: &AccountIdRef);
}

pub trait AccessControllableExt {
    async fn acl_grant_role(
        &self,
        contract_id: impl Into<AccountId>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome>;
}

impl AccessControllableExt for SigningNear {
    async fn acl_grant_role(
        &self,
        contract_id: impl Into<AccountId>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            contract_id,
            AccessControllable::acl_grant_role(&role.into(), account_id.as_ref())
                .gas(Gas::from_tgas(30)),
        )
        .await
    }
}
