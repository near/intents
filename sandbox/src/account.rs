use std::sync::Arc;

use impl_tools::autoimpl;
use near_api::{
    Account as NearApiAccount, Contract, NetworkConfig, SecretKey, Signer,
    signer::generate_secret_key, types::transaction::actions::FunctionCallAction,
};
use near_sdk::{
    AccountId, AccountIdRef, NearToken,
    serde::{Serialize, de::DeserializeOwned},
};

use crate::tx::TxBuilder;

const CONTRACT_DEPOSIT: NearToken = NearToken::from_near(100);

#[derive(Clone, Debug)]
pub struct Account {
    account_id: AccountId,
    network_config: NetworkConfig,
}

impl Account {
    pub const fn new(account_id: AccountId, network_config: NetworkConfig) -> Self {
        Self {
            account_id,
            network_config,
        }
    }

    pub const fn id(&self) -> &AccountId {
        &self.account_id
    }

    pub const fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    pub fn subaccount_id(&self, name: impl AsRef<str>) -> AccountId {
        format!("{}.{}", name.as_ref(), self.id())
            .parse()
            .expect("invalid subaccount name: must be a valid NEAR account ID")
    }

    pub fn subaccount_name(&self, account_id: &AccountIdRef) -> Option<String> {
        account_id
            .as_str()
            .strip_suffix(&format!(".{}", self.id()))
            .map(ToString::to_string)
    }

    pub async fn call_view_function_json<T>(
        &self,
        name: &str,
        args: impl Serialize,
    ) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        Contract(self.id().clone())
            .call_function(name, args)
            .read_only()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(Into::into)
    }

    pub async fn view(&self) -> anyhow::Result<near_api::types::Account> {
        self.view_account(self.id()).await
    }

    pub async fn view_account(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<near_api::types::Account> {
        NearApiAccount(account_id.into())
            .view()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(Into::into)
    }
}

#[autoimpl(Deref using self.account)]
#[derive(Clone)]
pub struct SigningAccount {
    account: Account,
    signer: Arc<Signer>,
    private_key: SecretKey,
}

impl SigningAccount {
    pub fn new(account: Account, secret_key: SecretKey) -> Self {
        Self {
            account,
            signer: Signer::from_secret_key(secret_key.clone()).unwrap(),
            private_key: secret_key,
        }
    }

    pub fn implicit(secret_key: SecretKey, network_config: NetworkConfig) -> Self {
        let public_key: defuse_crypto::PublicKey = secret_key.public_key().into();

        Self::new(
            Account::new(public_key.to_implicit_account_id(), network_config),
            secret_key,
        )
    }

    pub fn generate_implicit(network_config: NetworkConfig) -> Self {
        Self::implicit(generate_secret_key().unwrap(), network_config)
    }

    pub const fn account(&self) -> &Account {
        &self.account
    }

    pub fn signer(&self) -> Arc<Signer> {
        self.signer.clone()
    }

    pub const fn private_key(&self) -> &SecretKey {
        &self.private_key
    }

    pub fn tx(&self, receiver_id: AccountId) -> TxBuilder {
        TxBuilder::new(self.clone(), receiver_id)
    }

    pub async fn fund_implicit(&self, deposit: NearToken) -> anyhow::Result<Self> {
        let account = Self::generate_implicit(self.network_config.clone());

        self.tx(account.id().clone()).transfer(deposit).await?;

        Ok(account)
    }

    pub async fn transfer_near(
        &self,
        receiver_id: &AccountIdRef,
        deposit: NearToken,
    ) -> anyhow::Result<()> {
        self.tx(receiver_id.into()).transfer(deposit).await?;
        Ok(())
    }

    pub async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key()?;
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

    pub async fn deploy_contract(
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
