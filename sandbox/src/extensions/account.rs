use near_api::{
    Signer, signer::generate_secret_key, types::transaction::actions::FunctionCallAction,
};
use near_sdk::{AccountId, AccountIdRef, NearToken};

use crate::{Account, SigningAccount};

#[allow(async_fn_in_trait)]
pub trait ParentAccountExt {
    fn root_id(&self) -> &AccountIdRef;

    fn subaccount_id(&self, name: impl AsRef<str>) -> AccountId {
        format!("{}.{}", name.as_ref(), self.root_id())
            .parse()
            .unwrap()
    }

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
    ) -> anyhow::Result<SigningAccount>;
}

impl ParentAccountExt for SigningAccount {
    fn root_id(&self) -> &AccountIdRef {
        self.id()
    }

    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<SigningAccount> {
        let secret_key = generate_secret_key().unwrap();
        let public_key = secret_key.public_key();
        let subaccount = self.subaccount_id(name);

        let mut tx = self
            .tx(subaccount.clone())
            .create_account()
            .add_full_access_key(public_key);

        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }

        tx.await?;

        Ok(SigningAccount::new(
            Account::new(subaccount, self.network_config().clone()),
            Signer::new(Signer::from_secret_key(secret_key)).unwrap(),
        ))
    }
}

#[allow(async_fn_in_trait)]
pub trait AccountDeployerExt: ParentAccountExt {
    async fn deploy_contract(
        &self,
        name: &str,
        wasm: impl Into<Vec<u8>>,
        deposit: NearToken,
        init_args: Option<impl Into<FunctionCallAction>>,
    ) -> anyhow::Result<Account>;
}

impl AccountDeployerExt for SigningAccount {
    async fn deploy_contract(
        &self,
        name: &str,
        wasm: impl Into<Vec<u8>>,
        deposit: NearToken,
        init_args: Option<impl Into<FunctionCallAction>>,
    ) -> anyhow::Result<Account> {
        let subaccount = self.subaccount_id(name);

        // TODO: may be make optional?
        let mut tx = self
            .tx(subaccount.clone())
            .create_account()
            .transfer(deposit)
            .deploy(wasm.into());

        if let Some(args) = init_args {
            tx = tx.function_call(args);
        }

        tx.await?;

        Ok(Account::new(subaccount, self.network_config().clone()))
    }
}
