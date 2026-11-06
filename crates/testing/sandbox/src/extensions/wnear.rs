use anyhow::Result;
use near_kit::{
    AccountId, Gas, NearToken, SigningNear,
    protocol::Action,
    standards::{FungibleToken, FungibleTokenClient},
    transaction::FunctionCall,
};

use crate::{account::AccountExt, outcome::SuccessfulExecutionOutcome};

#[near_kit::contract]
pub trait WNear {
    #[call]
    fn near_deposit(&mut self);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn near_withdraw(&mut self, amount: NearToken);
}

pub trait WNearDeployerExt: Sized {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> FungibleTokenClient<Self>;
}

impl WNearDeployerExt for SigningNear {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> FungibleTokenClient<Self> {
        let account = self
            .create_subaccount(name, NearToken::from_near(100))
            .await;

        account
            .transaction(account.account_id())
            .add_action(Action::deploy_contract(wasm))
            .add_action(
                FunctionCall::new("new")
                    .args_raw(Vec::new())
                    .gas(Gas::from_tgas(10)),
            )
            .await
            .unwrap()
            .result()
            .unwrap();

        self.contract::<FungibleToken>(account.account_id())
    }
}

pub trait WNearExt {
    async fn near_deposit(
        &self,
        contract_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl WNearExt for SigningNear {
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
            .await?
            .try_into()
    }
}
