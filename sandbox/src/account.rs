use std::sync::Arc;

use anyhow::anyhow;
use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use impl_tools::autoimpl;
use near_api::{
    Account as NearApiAccount, Contract, NetworkConfig, PublicKey, Signer,
    signer::generate_secret_key,
    types::{
        Signature,
        transaction::actions::{GlobalContractDeployMode, GlobalContractIdentifier},
    },
};
use near_sdk::{
    AccountId, AccountIdRef, NearToken,
    account_id::ParseAccountError,
    serde::{Serialize, de::DeserializeOwned},
};

use crate::tx::{FnCallBuilder, TxBuilder};

#[autoimpl(AsRef using self.account_id)]
#[autoimpl(Deref using self.account_id)]
#[derive(Clone, Debug)]
pub struct Account {
    account_id: AccountId,
    network_config: NetworkConfig,
}

impl Account {
    #[inline]
    pub fn new(account_id: impl Into<AccountId>, network_config: NetworkConfig) -> Self {
        Self {
            account_id: account_id.into(),
            network_config,
        }
    }

    #[inline]
    pub const fn id(&self) -> &AccountId {
        &self.account_id
    }

    #[inline]
    pub const fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    #[inline]
    pub fn sub_account(&self, name: impl AsRef<str>) -> Result<Self, ParseAccountError> {
        Ok(Self {
            account_id: self.id().sub_account(name)?,
            network_config: self.network_config().clone(),
        })
    }

    pub async fn view(&self) -> anyhow::Result<near_api::types::Account> {
        NearApiAccount(self.id().clone())
            .view()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(Into::into)
    }

    pub async fn call_view_function_json<T>(
        &self,
        name: impl AsRef<str>,
        args: impl Serialize,
    ) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        Contract(self.id().clone())
            .call_function(name.as_ref(), args)
            .read_only()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(Into::into)
    }
}

impl AsRef<AccountIdRef> for Account {
    #[inline]
    fn as_ref(&self) -> &AccountIdRef {
        self.id()
    }
}

impl From<Account> for AccountId {
    #[inline]
    fn from(account: Account) -> Self {
        account.account_id
    }
}

#[autoimpl(Deref using self.account)]
#[derive(Clone)]
pub struct SigningAccount {
    account: Account,
    signer: Arc<Signer>,
}

impl SigningAccount {
    #[inline]
    pub const fn new(account: Account, signer: Arc<Signer>) -> Self {
        Self { account, signer }
    }

    pub fn generate_implicit(network_config: NetworkConfig) -> Self {
        let secret_key = generate_secret_key().unwrap();

        Self::new(
            Account::new(
                defuse_crypto::PublicKey::from(secret_key.public_key()).to_implicit_account_id(),
                network_config,
            ),
            Signer::from_secret_key(secret_key).unwrap(),
        )
    }

    #[inline]
    pub const fn account(&self) -> &Account {
        &self.account
    }

    #[inline]
    pub const fn signer(&self) -> &Arc<Signer> {
        &self.signer
    }

    #[inline]
    pub fn tx(&self, receiver_id: impl Into<AccountId>) -> TxBuilder {
        TxBuilder::new(self.clone(), receiver_id.into())
    }

    pub async fn fund_implicit(&self, deposit: NearToken) -> anyhow::Result<Self> {
        let account = Self::generate_implicit(self.network_config.clone());

        self.tx(account.id().clone()).transfer(deposit).await?;

        Ok(account)
    }

    pub async fn generate_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key().unwrap();
        let subaccount = self.sub_account(name)?;

        let mut tx = self.tx(subaccount.id()).create_account();
        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }
        tx.add_full_access_key(secret_key.public_key()).await?;

        Ok(Self::new(
            subaccount,
            Signer::from_secret_key(secret_key).unwrap(),
        ))
    }

    pub async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FnCallBuilder>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key().unwrap();
        let subaccount = self.sub_account(name)?;

        let mut tx = self
            .tx(subaccount.id())
            .create_account()
            .transfer(balance)
            .add_full_access_key(secret_key.public_key())
            .deploy(code);
        if let Some(init_call) = init_call.into() {
            tx = tx.function_call(init_call);
        }
        tx.await?;

        Ok(Self::new(
            subaccount,
            Signer::from_secret_key(secret_key).unwrap(),
        ))
    }

    pub async fn deploy_global_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key().unwrap();
        let subaccount = self.sub_account(name)?;

        self.tx(subaccount.id())
            .create_account()
            .transfer(balance)
            .add_full_access_key(secret_key.public_key())
            .deploy_global(code, GlobalContractDeployMode::AccountId)
            .await?;

        Ok(Self::new(
            subaccount,
            Signer::from_secret_key(secret_key).unwrap(),
        ))
    }

    pub async fn deploy_sub_contract_use_global(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        global_id: GlobalContractIdentifier,
        init_call: impl Into<Option<FnCallBuilder>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key().unwrap();
        let subaccount = self.sub_account(name)?;

        let mut tx = self
            .tx(subaccount.id())
            .create_account()
            .transfer(balance)
            .add_full_access_key(secret_key.public_key())
            .use_global(global_id);
        if let Some(init_call) = init_call.into() {
            tx = tx.function_call(init_call);
        }
        tx.await?;

        Ok(Self::new(
            subaccount,
            Signer::from_secret_key(secret_key).unwrap(),
        ))
    }

    pub async fn sign_nep413(&self, payload: Nep413Payload) -> anyhow::Result<SignedNep413Payload> {
        let pk = self.signer.get_public_key().await?;

        let (PublicKey::ED25519(pk), Signature::ED25519(sig)) = (
            pk,
            self.signer
                .sign_message_nep413(self.id().clone(), pk, &payload.clone().into())
                .await?,
        ) else {
            return Err(anyhow!("only ed25519 public keys are supported"));
        };

        Ok(SignedNep413Payload {
            payload,
            public_key: pk.0,
            signature: sig.to_bytes(),
        })
    }
}

impl From<SigningAccount> for Account {
    #[inline]
    fn from(account: SigningAccount) -> Self {
        account.account
    }
}
