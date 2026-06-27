use anyhow::Result;
use near_kit::{AccountId, Action, Final, FunctionCallAction, FungibleToken, Gas, Near, NearToken};
use serde::Serialize;

use crate::{account::Account, outcome::SuccessfulExecutionOutcome};

#[near_kit::contract]
pub trait WNear {
    #[call]
    fn near_deposit(&mut self);

    #[call]
    fn near_withdraw(&mut self, args: WNearAmount);
}

#[derive(Serialize)]
pub struct WNearAmount {
    amount: NearToken,
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
                gas: Gas::from_tgas(10),
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
            .add_action(
                WNear::near_deposit()
                    .deposit(amount)
                    .gas(Gas::from_tgas(10)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }
}
