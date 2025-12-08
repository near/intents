use near_api::{signer::generate_secret_key, types::transaction::actions::FunctionCallAction};
use near_sdk::{AccountId, AccountIdRef, NearToken};

use crate::{Account, SigningAccount};

const CONTRACT_DEPOSIT: NearToken = NearToken::from_near(100);

pub trait ParentAccountViewExt {
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
}

#[allow(async_fn_in_trait)]
pub trait ParentAccountExt: ParentAccountViewExt {
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

        Ok(Self::new(
            Account::new(subaccount, self.network_config().clone()),
            secret_key,
        ))
    }
}

impl ParentAccountViewExt for Account {
    fn root_id(&self) -> &AccountIdRef {
        self.id()
    }
}

impl ParentAccountViewExt for SigningAccount {
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

        // TODO: may be make optional?
        let mut tx = self
            .tx(subaccount.clone())
            .create_account()
            .transfer(CONTRACT_DEPOSIT)
            .deploy(wasm.into());

        if let Some(args) = init_args {
            tx = tx.function_call(args);
        }

        tx.await?;

        Ok(Account::new(subaccount, self.network_config().clone()))
    }
}
