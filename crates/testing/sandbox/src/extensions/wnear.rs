use anyhow::Result;
use near_kit::{Action, Final, FunctionCallAction, Near, NearToken};
use near_sdk::AccountId;

use crate::{account::Account, extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

#[near_kit::contract]
pub trait WNear {
    #[call]
    fn near_deposit(&mut self, amount: NearToken);

    #[call]
    fn near_withdraw(&mut self, amount: NearToken);
}

pub trait WNearDeployerExt {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> WNearClient;
}

impl WNearDeployerExt for Near {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> WNearClient {
        let account = self
            .create_subaccount(name, NearToken::from_near(100))
            .await;

        account
            .deploy(wasm)
            .add_action(Action::FunctionCall(FunctionCallAction {
                method_name: "new".to_string(),
                args: vec![],
                gas: DEFAULT_GAS,
                deposit: NearToken::from_near(0),
            }))
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .unwrap();

        self.contract::<WNear>(account.account_id())
    }
}

pub trait WNearExt {
    async fn near_deposit(
        &self,
        contract_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl WNearExt for Near {
    async fn near_deposit(
        &self,
        contract_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(&contract_id.into())
            .add_action(WNear::near_deposit(amount).deposit(amount))
            .wait_until(Final)
            .await?
            .try_into()
    }
}
