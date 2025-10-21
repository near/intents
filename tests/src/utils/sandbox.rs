use anyhow::Result;
use std::{fs, ops::Deref, path::Path};

use near_sdk::AccountId;
use near_workspaces::{Account, Contract, Network, Worker, types::NearToken};

pub fn read_wasm(path: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../"))
        .join(path)
        .with_extension("wasm");

    fs::read(filename).unwrap()
}
pub struct Sandbox {
    worker: Worker<near_workspaces::network::Sandbox>,
    root_account: Account,
}

#[allow(dead_code)]
impl Sandbox {
    pub async fn new() -> anyhow::Result<Self> {
        let worker = near_workspaces::sandbox().await?;
        let root_account = worker.root_account()?;

        Ok(Self {
            worker,
            root_account,
        })
    }

    pub async fn block_height(&self) -> u64 {
        self.worker.view_block().await.unwrap().height()
    }

    pub async fn skip_blocks(&self, num_blocks: u64) {
        self.worker.fast_forward(num_blocks).await.unwrap();
    }

    pub const fn worker(&self) -> &Worker<impl Network + 'static> {
        &self.worker
    }

    pub const fn root_account(&self) -> &Account {
        &self.root_account
    }

    pub async fn create_account(&self, name: &str) -> Result<Account> {
        self.root_account
            .create_new_subaccount(name, NearToken::from_near(10))
            .await
    }

    pub async fn account_exists(&self, name: &str) -> bool {
        let account_id = self.subaccount_id(name);

        self.worker.view_account(&account_id).await.is_ok()
    }
}

impl Deref for Sandbox {
    type Target = Account;

    fn deref(&self) -> &Self::Target {
        self.root_account()
    }
}

pub trait ParentAccount {
    fn subaccount_id(&self, name: &str) -> AccountId;

    fn subaccount_name(&self, account_id: &AccountId) -> String;

    async fn create_new_subaccount(
        &self,
        name: &str,
        balance: NearToken,
    ) -> anyhow::Result<Account>;
}

impl ParentAccount for near_workspaces::Account {
    fn subaccount_name(&self, account_id: &AccountId) -> String {
        account_id
            .as_str()
            .strip_suffix(&format!(".{}", self.id()))
            .unwrap()
            .to_string()
    }

    fn subaccount_id(&self, name: &str) -> AccountId {
        format!("{}.{}", name, self.id()).parse().unwrap()
    }

    async fn create_new_subaccount(&self, name: &str, balance: NearToken) -> anyhow::Result<Self> {
        self.create_subaccount(name)
            .initial_balance(balance)
            .transact()
            .await
            .map(|result| result.result)
            .map_err(Into::into)
    }
}

impl ParentAccount for Contract {
    fn subaccount_name(&self, account_id: &AccountId) -> String {
        self.as_account().subaccount_name(account_id)
    }

    fn subaccount_id(&self, name: &str) -> AccountId {
        self.as_account().subaccount_id(name)
    }

    async fn create_new_subaccount(
        &self,
        name: &str,
        balance: NearToken,
    ) -> anyhow::Result<Account> {
        self.as_account().create_new_subaccount(name, balance).await
    }
}
