use crate::{SigningAccount, anyhow, tx::FnCallBuilder};
use defuse::arbitrary::ArbitraryAction;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

pub trait ArbitraryManagerExt {
    async fn arbitrary_call(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
        action: ArbitraryAction,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess>;
}

impl ArbitraryManagerExt for SigningAccount {
    async fn arbitrary_call(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
        action: ArbitraryAction,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("arbitrary_call")
                    .json_args(json!({
                        "account_id": account_id.as_ref(),
                        "action": action,
                    }))
                    .with_deposit(deposit),
            )
            .await
    }
}
