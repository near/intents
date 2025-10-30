use std::{fs, path::Path, sync::Arc};

use futures::{FutureExt, future::BoxFuture};
use impl_tools::autoimpl;
use near_api::{
    Contract, NetworkConfig, PublicKey, SecretKey, Signer, Transaction,
    errors::ExecuteTransactionError,
    signer::generate_secret_key,
    types::{
        AccessKey, AccessKeyPermission, Action,
        transaction::{
            actions::{
                AddKeyAction, CreateAccountAction, DeployContractAction,
                DeployGlobalContractAction, FunctionCallAction, GlobalContractDeployMode,
                GlobalContractIdentifier, TransferAction, UseGlobalContractAction,
            },
            result::ExecutionFinalResult,
        },
    },
};
use near_sdk::{
    AccountId, Gas, NearToken,
    serde::{Serialize, de::DeserializeOwned},
    serde_json,
};

#[derive(Clone)]
pub struct Account {
    account_id: AccountId,
    network_config: NetworkConfig,
}

impl Account {
    pub fn new(account_id: AccountId, network_config: NetworkConfig) -> Self {
        Self {
            account_id,
            network_config,
        }
    }

    pub fn id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    pub fn subaccount(&self, name: impl AsRef<str>) -> Self {
        Self::new(
            format!("{}.{}", name.as_ref(), self.id()).parse().unwrap(),
            self.network_config.clone(),
        )
    }

    pub async fn call_function_json<T>(&self, name: &str, args: impl Serialize) -> T
    where
        T: DeserializeOwned + Send + Sync,
    {
        Contract(self.id().clone())
            .call_function(name, args)
            .unwrap()
            .read_only()
            .fetch_from(&self.network_config)
            .await
            .unwrap()
            .data
    }
}

#[autoimpl(Deref using self.account)]
#[derive(Clone)]
pub struct SigningAccount {
    account: Account,
    signer: Arc<Signer>,
}

impl SigningAccount {
    pub fn new(account: Account, signer: Arc<Signer>) -> Self {
        Self { account, signer }
    }

    pub fn implicit(secret_key: SecretKey, network_config: NetworkConfig) -> Self {
        let public_key: defuse::core::crypto::PublicKey = secret_key.public_key().into();

        Self::new(
            Account::new(public_key.to_implicit_account_id(), network_config),
            Signer::new(Signer::from_secret_key(secret_key)).unwrap(),
        )
    }

    pub fn generate_implicit(network_config: NetworkConfig) -> Self {
        Self::implicit(generate_secret_key().unwrap(), network_config)
    }

    pub fn signer(&self) -> Arc<Signer> {
        self.signer.clone()
    }

    pub fn tx(&self, receiver_id: AccountId) -> TxBuilder {
        TxBuilder::new(self.clone(), receiver_id)
    }

    pub async fn fund_implicit(&self, deposit: NearToken) -> Self {
        let account = Self::generate_implicit(self.network_config.clone());

        self.tx(account.id().clone())
            .transfer(deposit)
            .await
            .unwrap()
            .into_result()
            .unwrap();

        account
    }

    pub async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Self {
        let secret_key = generate_secret_key().unwrap();
        let public_key = secret_key.public_key();

        let subaccount = Self::new(
            self.subaccount(name),
            Signer::new(Signer::from_secret_key(secret_key)).unwrap(),
        );

        let mut tx = self.tx(subaccount.id().clone()).create_account();
        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }
        tx.add_full_access_key(public_key)
            .await
            .unwrap()
            .into_result()
            .unwrap();

        subaccount
    }
}

pub struct TxBuilder {
    signer: SigningAccount,
    receiver_id: AccountId,

    actions: Vec<Action>,
}

impl TxBuilder {
    pub const fn new(signer: SigningAccount, receiver_id: AccountId) -> Self {
        Self {
            signer,
            receiver_id,
            actions: Vec::new(),
        }
    }

    pub fn create_account(self) -> Self {
        self.add_action(Action::CreateAccount(CreateAccountAction {}))
    }

    pub fn transfer(self, deposit: NearToken) -> Self {
        self.add_action(Action::Transfer(TransferAction { deposit }))
    }

    pub fn deploy(self, code: Vec<u8>) -> Self {
        self.add_action(Action::DeployContract(DeployContractAction { code }))
    }

    pub fn deploy_global(self, code: Vec<u8>, deploy_mode: GlobalContractDeployMode) -> Self {
        self.add_action(Action::DeployGlobalContract(DeployGlobalContractAction {
            code,
            deploy_mode,
        }))
    }

    pub fn use_global(self, global_id: GlobalContractIdentifier) -> Self {
        self.add_action(Action::UseGlobalContract(
            UseGlobalContractAction {
                contract_identifier: global_id,
            }
            .into(),
        ))
    }

    pub fn add_full_access_key(self, pk: impl Into<PublicKey>) -> Self {
        self.add_key(
            pk,
            AccessKey {
                nonce: 0.into(),
                permission: AccessKeyPermission::FullAccess,
            },
        )
    }

    fn add_key(self, pk: impl Into<PublicKey>, access_key: AccessKey) -> Self {
        self.add_action(Action::AddKey(
            AddKeyAction {
                public_key: pk.into(),
                access_key,
            }
            .into(),
        ))
    }

    pub fn function_call_json(
        self,
        name: impl Into<String>,
        args: impl Serialize,
        gas: Gas,
        deposit: NearToken,
    ) -> Self {
        self.add_action(Action::FunctionCall(
            FunctionCallAction {
                method_name: name.into(),
                args: serde_json::to_vec(&args).unwrap(),
                gas,
                deposit,
            }
            .into(),
        ))
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn add_actions(mut self, actions: impl IntoIterator<Item = Action>) -> Self {
        self.actions.extend(actions);
        self
    }
}

impl IntoFuture for TxBuilder {
    type Output = Result<ExecutionFinalResult, ExecuteTransactionError>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            Transaction::construct(self.signer.id().clone(), self.receiver_id)
                .add_actions(self.actions)
                .with_signer(self.signer.signer())
                .send_to(self.signer.network_config())
                .await
        }
        .boxed()
    }
}

pub(super) fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename).unwrap()
}
