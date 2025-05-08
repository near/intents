use std::sync::LazyLock;

use near_sdk::{AccountId, NearToken};
use near_workspaces::Contract;
use serde_json::json;

use crate::utils::{Sandbox, account::AccountExt, nft::NftExt, read_wasm};

static ACCOUNT_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_account_contract"));

pub trait AccountShardExt {
    async fn deploy_account_shard(
        &self,
        account_shard_id: &str,
        owner: Option<AccountId>,
    ) -> anyhow::Result<Contract>;

    async fn create_account(
        &self,
        account_shard_id: &AccountId,
        derivation_path: &str,
        owner: Option<AccountId>,
    ) -> anyhow::Result<()>;
}

impl AccountShardExt for near_workspaces::Account {
    async fn deploy_account_shard(
        &self,
        account_shard_id: &str,
        owner: Option<AccountId>,
    ) -> anyhow::Result<Contract> {
        let contract = self
            .deploy_contract(account_shard_id, &ACCOUNT_WASM)
            .await?;

        contract
            .call("new")
            .args_json(json!({
                "owner": owner.unwrap_or_else(|| self.id().clone()),
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(contract)
    }

    async fn create_account(
        &self,
        account_shard_id: &AccountId,
        derivation_path: &str,
        owner: Option<AccountId>,
    ) -> anyhow::Result<()> {
        self.call(account_shard_id, "create_account")
            .args_json(json!({
                "derivation_path": derivation_path,
                "owner": owner,
            }))
            .deposit(NearToken::from_millinear(2))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }
}

#[tokio::test]
async fn account_shard() {
    let sandbox = Sandbox::new().await.unwrap();

    let dao = sandbox
        .create_subaccount("dao", NearToken::from_near(100))
        .await
        .unwrap();
    let account_shard = dao
        .deploy_account_shard("account-shard", None)
        .await
        .unwrap();

    let user1 = sandbox
        .create_subaccount("user1", NearToken::from_near(10))
        .await
        .unwrap();
    let user2 = sandbox
        .create_subaccount("user2", NearToken::from_near(10))
        .await
        .unwrap();

    user1
        .create_account(account_shard.id(), "a", None)
        .await
        .unwrap();
    assert_eq!(
        &account_shard
            .as_account()
            .self_nft_token(&"a".to_string())
            .await
            .unwrap()
            .unwrap()
            .owner_id,
        user1.id()
    );

    user1
        .nft_transfer(account_shard.id(), user2.id(), "a".to_string(), None)
        .await
        .unwrap();
    assert_eq!(
        &account_shard
            .as_account()
            .self_nft_token(&"a".to_string())
            .await
            .unwrap()
            .unwrap()
            .owner_id,
        user2.id()
    );
}
