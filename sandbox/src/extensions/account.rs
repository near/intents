use near_api::{Signer, signer::generate_secret_key};
use near_sdk::{AccountIdRef, Gas, NearToken, serde::Serialize};

use crate::{Account, SigningAccount, TxResult};

pub struct JsonFunctionCallArgs<T: Serialize> {
    pub name: &'static str,
    pub args: T,
}

pub trait AccountDeployerExt {
    async fn deploy_contract<T: Serialize>(
        &self,
        name: &str,
        wasm: impl Into<Vec<u8>>,
        init_args: Option<JsonFunctionCallArgs<T>>,
    ) -> anyhow::Result<Account>;
}

impl AccountDeployerExt for SigningAccount {
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

pub trait ParentAccount {
    fn root_id(&self) -> &AccountIdRef;

    fn subaccount(&self, name: impl AsRef<str>) -> Account;

    fn subaccount_name(&self, account_id: &AccountIdRef) -> String {
        account_id
            .as_str()
            .strip_suffix(&format!(".{}", self.root_id()))
            .unwrap()
            .to_string()
    }

    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> TxResult<SigningAccount>;
}

impl ParentAccount for SigningAccount {
    fn root_id(&self) -> &AccountIdRef {
        self.id()
    }

    fn subaccount(&self, name: impl AsRef<str>) -> Account {
        let acc = self.account();

        Account::new(
            format!("{}.{}", name.as_ref(), acc.id()).parse().unwrap(),
            acc.network_config().clone(),
        )
    }

    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> TxResult<SigningAccount> {
        // TODO: do we need to add separate keys?
        let secret_key = generate_secret_key().unwrap();
        let public_key = secret_key.public_key();
        let subaccount = self.subaccount(name);

        let mut tx = self.tx(subaccount.id().clone()).create_account();
        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }
        tx.add_full_access_key(public_key).await?;

        Ok(SigningAccount::new(
            subaccount,
            Signer::new(Signer::from_secret_key(secret_key)).unwrap(),
        ))
    }
}
