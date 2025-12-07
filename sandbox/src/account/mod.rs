mod mt;

pub use self::mt::*;

use std::sync::Arc;

use impl_tools::autoimpl;
use near_api::{
    Account as NearApiAccount, Contract, NetworkConfig, SecretKey, Signer,
    signer::generate_secret_key,
};
use near_sdk::{
    AccountId, NearToken,
    serde::{Serialize, de::DeserializeOwned},
};

use crate::{TxBuilder, TxResult};

#[derive(Clone)]
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

    #[must_use]
    pub fn subaccount(&self, name: impl AsRef<str>) -> Self {
        Self::new(
            format!("{}.{}", name.as_ref(), self.id()).parse().unwrap(),
            self.network_config.clone(),
        )
    }

    pub async fn call_function_json<T>(&self, name: &str, args: impl Serialize) -> anyhow::Result<T>
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
        NearApiAccount(self.id().clone())
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
}

impl SigningAccount {
    pub const fn new(account: Account, signer: Arc<Signer>) -> Self {
        Self { account, signer }
    }

    pub fn implicit(secret_key: SecretKey, network_config: NetworkConfig) -> Self {
        let public_key: defuse_crypto::PublicKey = secret_key.public_key().into();

        Self::new(
            Account::new(public_key.to_implicit_account_id(), network_config),
            Signer::from_secret_key(secret_key).unwrap(),
        )
    }

    pub fn generate_implicit(network_config: NetworkConfig) -> Self {
        Self::implicit(generate_secret_key().unwrap(), network_config)
    }

    pub fn signer(&self) -> Arc<Signer> {
        self.signer.clone()
    }

    pub fn id(&self) -> AccountId {
        self.account.id().clone()
    }

    pub fn tx(&self, receiver_id: AccountId) -> TxBuilder {
        TxBuilder::new(self.clone(), receiver_id)
    }

    pub async fn fund_implicit(&self, deposit: NearToken) -> TxResult<Self> {
        let account = Self::generate_implicit(self.network_config.clone());

        self.tx(account.id().clone()).transfer(deposit).await?;

        Ok(account)
    }

    pub async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> TxResult<Self> {
        let secret_key = generate_secret_key().unwrap();
        let public_key = secret_key.public_key();

        let subaccount = Self::new(
            self.subaccount(name),
            Signer::from_secret_key(secret_key).unwrap(),
        );

        let mut tx = self.tx(subaccount.id().clone()).create_account();
        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }
        tx.add_full_access_key(public_key).await?;

        Ok(subaccount)
    }
}
