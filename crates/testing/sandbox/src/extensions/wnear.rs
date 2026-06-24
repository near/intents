use anyhow::Result;
use near_kit::{AccountId, Action, Final, FunctionCallAction, FungibleToken, Near, NearToken};

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
    ) -> FungibleToken;
}

impl WNearDeployerExt for Near {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> FungibleToken {
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

        self.ft(account.account_id()).unwrap()
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
        self.transaction(contract_id.into())
            .add_action(WNear::near_deposit(amount).deposit(amount))
            .wait_until(Final)
            .await?
            .try_into()
    }
}
