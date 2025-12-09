use std::sync::Arc;

use impl_tools::autoimpl;
use near_api::{
    Account as NearApiAccount, Contract, NetworkConfig, SecretKey, Signer,
    signer::generate_secret_key,
};
use near_sdk::{
    AccountId, AccountIdRef, NearToken,
    serde::{Serialize, de::DeserializeOwned},
};

use crate::tx::TxBuilder;

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

    // TODO maybe separate it to another trait?
    pub async fn call_view_function_json<T>(
        &self,
        name: &str,
        args: impl Serialize,
    ) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        Contract(self.id().clone())
            .call_function(name, args)?
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
            signer: Signer::new(Signer::from_secret_key(secret_key.clone())).unwrap(),
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
}
