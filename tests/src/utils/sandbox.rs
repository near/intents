use std::{fs, ops::Deref, path::Path};

use near_workspaces::{Account, Network, Worker, types::NearToken};

pub fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
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

    pub async fn create_subaccount(
        &self,
        name: &str,
        balance: NearToken,
    ) -> anyhow::Result<Account> {
        self.root_account
            .create_subaccount(name)
            .initial_balance(balance)
            .transact()
            .await
            .map(|result| result.result)
            .map_err(Into::into)
    }

    pub async fn create_account(&self, name: &str) -> Account {
        self.create_subaccount(name, NearToken::from_near(10))
            .await
            .unwrap()
    }
}

impl Deref for Sandbox {
    type Target = Account;

    fn deref(&self) -> &Self::Target {
        self.root_account()
    }
}
