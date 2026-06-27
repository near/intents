use anyhow::Result;
use near_kit::{AccountId, AccountIdRef, Gas, Near};
use serde::Serialize;

use crate::{extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

#[near_kit::contract]
pub trait AccessControllable {
    fn acl_has_role(&self, args: AclRoleArgs) -> bool;
    #[call]
    fn acl_grant_role(&mut self, args: AclRoleArgs) -> Option<bool>;
}

#[derive(Serialize)]
pub struct AclRoleArgs<'a> {
    pub role: &'a str,
    pub account_id: &'a AccountIdRef,
}

pub trait AccessControllableExt {
    async fn acl_grant_role(
        &self,
        contract_id: impl Into<AccountId>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl AccessControllableExt for Near {
    async fn acl_grant_role(
        &self,
        contract_id: impl Into<AccountId>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            contract_id,
            AccessControllable::acl_grant_role(AclRoleArgs {
                role: &role.into(),
                account_id: account_id.as_ref(),
            })
            .gas(Gas::from_tgas(30)),
        )
        .await
    }
}
