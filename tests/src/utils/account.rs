#![allow(dead_code)]

use defuse_sandbox::{Account, SigningAccount};
use near_sdk::{Gas, NearToken, serde::Serialize};

pub struct JsonFunctionCallArgs<T: Serialize> {
    pub name: &'static str,
    pub args: T,
}

pub trait AccountExt {
    async fn deploy_contract<T: Serialize>(
        &self,
        name: &str,
        wasm: impl Into<Vec<u8>>,
        init_args: Option<JsonFunctionCallArgs<T>>,
    ) -> anyhow::Result<Account>;
}

impl AccountExt for SigningAccount {
    async fn deploy_contract<T: Serialize>(
        &self,
        name: &str,
        wasm: impl Into<Vec<u8>>,
        init_args: Option<JsonFunctionCallArgs<T>>,
    ) -> anyhow::Result<Account> {
        let account_id = self.subaccount(name).id().clone();

        let mut tx = self
            .tx(account_id.clone())
            .create_account()
            .transfer(NearToken::from_near(15))
            .deploy(wasm.into());

        if let Some(args) = init_args {
            tx = tx.function_call_json::<()>(
                args.name,
                args.args,
                Gas::from_tgas(10),
                NearToken::from_yoctonear(0),
            );
        }

        tx.no_result().await?;

        Ok(Account::new(account_id, self.network_config().clone()))
    }
}
