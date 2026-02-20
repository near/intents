use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use impl_tools::autoimpl;
use near_api::{
    Account as NearApiAccount, Contract, CryptoHash, NetworkConfig, PublicKey, SecretKey, Signer,
    signer::generate_secret_key,
    types::{
        Signature,
        account::ContractState,
        transaction::actions::{GlobalContractDeployMode, GlobalContractIdentifier},
    },
};
use near_sdk::{
    AccountId, AccountIdRef, GlobalContractId, NearToken,
    account_id::ParseAccountError,
    serde::{Serialize, de::DeserializeOwned},
    state_init::StateInit,
};
use tracing::instrument;

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

    pub async fn call_view_function_raw(
        &self,
        name: impl AsRef<str>,
        args: impl Serialize,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(Contract(self.id().clone())
            .call_function(name.as_ref(), args)
            .read_only_raw()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)?)
    }

    pub async fn global_contract_id(&self) -> anyhow::Result<GlobalContractId> {
        let account = self.view().await?;
        match account.contract_state {
            ContractState::GlobalHash(hash) => Ok(GlobalContractId::CodeHash(hash.0.into())),
            ContractState::GlobalAccountId(id) => Ok(GlobalContractId::AccountId(id)),
            other => anyhow::bail!("unexpected contract state: {other:?}"),
        }
    }

    pub async fn global_contract_wasm(&self) -> anyhow::Result<Vec<u8>> {
        use near_sdk::base64::{Engine as _, engine::general_purpose::STANDARD};
        let id = self.global_contract_id().await?;
        let code_view = match &id {
            GlobalContractId::CodeHash(hash) => {
                Contract::global_wasm()
                    .by_hash(CryptoHash(*hash.as_ref()))
                    .fetch_from(&self.network_config)
                    .await?
            }
            GlobalContractId::AccountId(account_id) => {
                Contract::global_wasm()
                    .by_account_id(account_id.clone())
                    .fetch_from(&self.network_config)
                    .await?
            }
        };
        STANDARD
            .decode(&code_view.data.code_base64)
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

#[autoimpl(Debug ignore self.signer)]
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

    pub fn generate_implicit(network_config: NetworkConfig) -> Result<Self> {
        let secret_key = generate_secret_key().context("failed to generate secret key")?;

        Ok(Self::new(
            Account::new(
                defuse_crypto::PublicKey::from(secret_key.public_key()).to_implicit_account_id(),
                network_config,
            ),
            Signer::from_secret_key(secret_key).context("failed to create signer")?,
        ))
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

    #[inline]
    pub async fn state_init(
        &self,
        state_init: StateInit,
        deposit: NearToken,
    ) -> anyhow::Result<AccountId> {
        let deterministic_account_id = state_init.derive_account_id();
        self.tx(deterministic_account_id.clone())
            .state_init(state_init, deposit)
            .await?;
        Ok(deterministic_account_id)
    }

    pub async fn fund_implicit(&self, deposit: NearToken) -> anyhow::Result<Self> {
        let account = Self::generate_implicit(self.network_config.clone())?;

        self.tx(account.id().clone()).transfer(deposit).await?;

        Ok(account)
    }

    fn generate_keys(pk_num: usize) -> Result<Vec<SecretKey>> {
        if pk_num < 1 {
            anyhow::bail!("pk_num must be at least 1");
        }

        (0..pk_num)
            .map(|_| generate_secret_key().context("failed to generate secret key"))
            .collect::<Result<Vec<_>>>()
    }

    async fn add_keys_to_signer_pool(
        &self,
        tx: TxBuilder,
        pks: impl IntoIterator<Item = SecretKey>,
    ) -> Result<()> {
        let pks: Vec<_> = pks.into_iter().collect();

        pks.iter()
            .map(SecretKey::public_key)
            .fold(tx, TxBuilder::add_full_access_key)
            .await?;

        futures::future::try_join_all(
            pks.into_iter()
                .map(|secret_key| self.signer.add_secret_key_to_pool(secret_key)),
        )
        .await?;

        Ok(())
    }

    pub async fn extend_signer(&self, pk_num: usize) -> Result<()> {
        let pks = Self::generate_keys(pk_num)?;

        self.add_keys_to_signer_pool(self.tx(self.id()), pks).await
    }

    #[instrument(skip_all, fields(name = name.as_ref()))]
    pub async fn generate_subaccount_highload(
        &self,
        name: impl AsRef<str>,
        pk_num: usize,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<Self> {
        let subaccount = self.sub_account(name)?;
        let pks = Self::generate_keys(pk_num)?;

        let mut tx = self.tx(subaccount.id()).create_account();
        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }

        // pks.iter()
        //     .map(SecretKey::public_key)
        //     .fold(tx, TxBuilder::add_full_access_key)
        //     .await?;

        let signer = Self::new(
            subaccount,
            Signer::from_secret_key(
                pks.first()
                    .cloned()
                    .context("generated key list is empty")?,
            )?,
        );

        signer.add_keys_to_signer_pool(tx, pks).await?;

        Ok(signer)
    }

    #[instrument(skip_all, fields(name = name.as_ref()))]
    pub async fn generate_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> anyhow::Result<Self> {
        self.generate_subaccount_highload(name, 1, balance).await
    }

    pub async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FnCallBuilder>>,
    ) -> anyhow::Result<Self> {
        let secret_key = generate_secret_key().context("Failed to generate secret key")?;
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

    pub async fn deploy_global_contract(
        &self,
        code: impl Into<Vec<u8>>,
        mode: GlobalContractDeployMode,
    ) -> anyhow::Result<()> {
        self.tx(self.id()).deploy_global(code, mode).await?;
        Ok(())
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
