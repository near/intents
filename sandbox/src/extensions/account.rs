use near_api::types::transaction::actions::FunctionCallAction;
use near_sdk::{AccountId, AccountIdRef, NearToken};

use crate::{Account, SigningAccount};

const CONTRACT_DEPOSIT: NearToken = NearToken::from_near(100);

pub trait ParentAccountViewExt {
    fn root_id(&self) -> &AccountIdRef;

    fn subaccount_id(&self, name: impl AsRef<str>) -> AccountId {
        format!("{}.{}", name.as_ref(), self.root_id())
            .parse()
            .expect("invalid subaccount name: must be a valid NEAR account ID")
    }

    fn subaccount_name(&self, account_id: &AccountIdRef) -> Option<String> {
        account_id
            .as_str()
            .strip_suffix(&format!(".{}", self.root_id()))
            .map(ToString::to_string)
    }
}

#[allow(async_fn_in_trait)]
pub trait ParentAccountExt {
    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<SigningAccount>;
}

impl ParentAccountExt for SigningAccount {
    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<Self> {
        // NOTE: subaccounts are created with the same key as the parent account
        let secret_key = self.private_key().clone();
        let public_key = secret_key.public_key();
        let subaccount = self.subaccount_id(name);

        let account = Self::new(
            Account::new(subaccount.clone(), self.network_config().clone()),
            secret_key,
        );

        let exist = account.view().await.is_ok_and(|v| v.storage_usage > 0);

        if !exist {
            let mut tx = self
                .tx(subaccount)
                .create_account()
                .add_full_access_key(public_key);

            if let Some(balance) = balance.into() {
                tx = tx.transfer(balance);
            }

            tx.await?;
        }

        Ok(account)
    }
}

impl ParentAccountViewExt for Account {
    fn root_id(&self) -> &AccountIdRef {
        self.id()
    }
}

#[allow(async_fn_in_trait)]
pub trait AccountDeployerExt: ParentAccountExt {
    async fn deploy_contract(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
        init_args: Option<impl Into<FunctionCallAction>>,
    ) -> anyhow::Result<Account>;
}

impl AccountDeployerExt for SigningAccount {
    async fn deploy_contract(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
        init_args: Option<impl Into<FunctionCallAction>>,
    ) -> anyhow::Result<Account> {
        let subaccount = self.subaccount_id(name);

        let mut tx = self
            .tx(subaccount.clone())
            .create_account()
            .add_full_access_key(self.private_key().public_key())
            .transfer(CONTRACT_DEPOSIT)
            .deploy(wasm.into());

        if let Some(args) = init_args {
            tx = tx.function_call(args);
        }

        tx.await?;

        Ok(Account::new(subaccount, self.network_config().clone()))
    }
}
