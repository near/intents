use defuse::contract::Role;
use near_sdk::{AccountId, NearToken, PublicKey};
use near_workspaces::{Account, types::SecretKey};
use rstest::rstest;
use serde_json::json;

use crate::{
    tests::defuse::{env::Env, intents::ExecuteIntentsExt},
    utils::acl::AclExt,
};

#[tokio::test]
#[rstest]
async fn test_relayer_keys(#[values(false, true)] no_registration: bool) {
    use near_workspaces::Contract;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(no_registration)
        .build()
        .await;

    env.acl_grant_role(env.defuse.id(), Role::RelayerKeysManager, env.user1.id())
        .await
        .unwrap();

    let worker = env.sandbox().worker().clone();

    // We generate a new key, because all keys generated by Near Workspaces are the same
    let new_relayer_secret_key = SecretKey::from_random(near_workspaces::types::KeyType::ED25519);
    let new_relayer_public_key = new_relayer_secret_key.public_key();
    let new_relayer_public_key_near_sdk = new_relayer_public_key.to_string().parse().unwrap();

    // Attempt to use the key that we still didn't add, to execute an intent, which fails
    assert!(
        Contract::from_secret_key(
            env.defuse.id().clone(),
            new_relayer_secret_key.clone(),
            &worker,
        )
        .execute_intents([]) // Empty because it's just to ensure that authorization works/doesn't work
        .await
        .unwrap_err()
        .to_string()
        .contains("Failed to query access key")
    );

    // A random, unauthorized user attempts to add a key (no role `Role::RelayerKeysManager`) and fails
    assert!(
        env.user2
            .add_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
            .await
            .unwrap_err()
            .to_string()
            .contains("Requires one of these roles:")
    );

    // A `Role::RelayerKeysManager` attempts to add the key, successfully
    env.user1
        .add_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
        .await
        .unwrap();

    // Attempt to add a key that already exists
    assert!(
        env.user1
            .add_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
            .await
            .unwrap_err()
            .to_string()
            .contains("key already exists")
    );

    // Create a Function-call Key, then use it to execute an (empty) intent
    Contract::from_secret_key(
        env.defuse.id().clone(),
        new_relayer_secret_key.clone(),
        &worker,
    )
    .execute_intents([]) // Empty because it's just to ensure that authorization works/doesn't work
    .await
    .unwrap();

    // Attempt to delete the key by an unauthorized user
    assert!(
        env.user2
            .delete_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
            .await
            .unwrap_err()
            .to_string()
            .contains("Requires one of these roles:")
    );

    // Delete the relayer key by the authorized entity
    env.user1
        .delete_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
        .await
        .unwrap();

    // Delete the same key again, which won't work
    assert!(
        env.user1
            .delete_relayer_key(env.defuse.id(), &new_relayer_public_key_near_sdk)
            .await
            .unwrap_err()
            .to_string()
            .contains("key not found")
    );

    let access_keys = env.defuse.view_access_keys().await.unwrap();
    dbg!(&access_keys);
    assert!(!access_keys.is_empty());
}

pub trait RelayerKeysExt {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;
}

impl RelayerKeysExt for Account {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "add_relayer_key")
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

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
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
