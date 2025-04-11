use defuse::contract::Role;
use near_sdk::{AccountId, NearToken, PublicKey};
use near_workspaces::Account;
use rstest::rstest;
use serde_json::json;

use crate::{tests::defuse::env::Env, utils::acl::AclExt};

#[tokio::test]
#[rstest]
async fn test_relayer_keys(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(no_registration)
        .build()
        .await;

    env.acl_grant_role(env.defuse.id(), Role::RelayerKeysManager, env.user1.id())
        .await
        .unwrap();

    env.user1
        .add_relayer_key(
            env.defuse.id(),
            NearToken::from_near(1),
            env.defuse
                .as_account()
                .secret_key()
                .public_key()
                .to_string()
                .parse()
                .unwrap(),
        )
        .await
        .unwrap_err();

    env.user1
        .delete_relayer_key(
            env.defuse.id(),
            env.defuse
                .as_account()
                .secret_key()
                .public_key()
                .to_string()
                .parse()
                .unwrap(),
        )
        .await
        .unwrap_err();

    let access_keys = env.defuse.view_access_keys().await.unwrap();
    dbg!(&access_keys);
    assert!(!access_keys.is_empty());
}

pub trait RelayerKeysExt {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        allowance: NearToken,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;
}

impl RelayerKeysExt for Account {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        allowance: NearToken,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "add_relayer_key")
            .deposit(allowance)
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "delete_relayer_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }
}
