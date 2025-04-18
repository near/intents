use std::time::Duration;

use defuse::{
    contract::Role,
    core::{
        Deadline,
        crypto::PublicKey,
        intents::{
            DefuseIntents,
            tokens::{TokenAmounts, Transfer},
        },
        tokens::TokenId,
    },
};
use near_sdk::{AccountId, AccountIdRef, NearToken};
use randomness::Rng;
use rstest::rstest;
use serde_json::json;
use test_utils::random::{Seed, make_seedable_rng, random_seed};

use crate::{
    tests::defuse::{DefuseSigner, env::Env, intents::ExecuteIntentsExt},
    utils::mt::MtExt,
};

pub trait AccountManagerExt {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;
}

impl AccountManagerExt for near_workspaces::Account {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "add_public_key")
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

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "remove_public_key")
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

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.view(defuse_contract_id, "has_public_key")
            .args_json(json!({
                "account_id": account_id,
                "public_key": public_key,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.defuse_has_public_key(self.id(), account_id, public_key)
            .await
    }
}

impl AccountManagerExt for near_workspaces::Contract {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.as_account()
            .add_public_key(defuse_contract_id, public_key)
            .await
    }

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.as_account()
            .remove_public_key(defuse_contract_id, public_key)
            .await
    }

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .defuse_has_public_key(defuse_contract_id, account_id, public_key)
            .await
    }

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .has_public_key(account_id, public_key)
            .await
    }
}

pub trait AccountForceLockerExt {
    async fn is_account_locked(&self, account_id: &AccountIdRef) -> anyhow::Result<bool>;
    async fn defuse_is_account_locked(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn force_lock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool>;
    async fn defuse_force_lock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;
    async fn force_unlock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool>;
    async fn defuse_force_unlock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;
}

impl AccountForceLockerExt for near_workspaces::Account {
    async fn is_account_locked(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.defuse_is_account_locked(self.id(), account_id).await
    }

    async fn defuse_is_account_locked(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.view(defuse_contract_id, "is_account_locked")
            .args_json(json!({
                "account_id": account_id,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn force_lock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.defuse_force_lock_account(self.id(), account_id).await
    }

    async fn defuse_force_lock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.call(defuse_contract_id, "force_lock_account")
            .args_json(json!({
                "account_id": account_id,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn force_unlock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.defuse_force_unlock_account(self.id(), account_id)
            .await
    }

    async fn defuse_force_unlock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.call(defuse_contract_id, "force_unlock_account")
            .args_json(json!({
                "account_id": account_id,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }
}

impl AccountForceLockerExt for near_workspaces::Contract {
    async fn is_account_locked(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.as_account().is_account_locked(account_id).await
    }

    async fn defuse_is_account_locked(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .defuse_is_account_locked(defuse_contract_id, account_id)
            .await
    }

    async fn force_lock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.as_account().force_lock_account(account_id).await
    }

    async fn defuse_force_lock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .defuse_force_lock_account(defuse_contract_id, account_id)
            .await
    }

    async fn force_unlock_account(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.as_account().force_unlock_account(account_id).await
    }

    async fn defuse_force_unlock_account(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .defuse_force_unlock_account(defuse_contract_id, account_id)
            .await
    }
}

#[tokio::test]
#[rstest]
#[trace]
async fn test_lock_account(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder()
        .self_as_grantee(Role::UnrestrictedAccountLocker)
        .self_as_grantee(Role::UnrestrictedAccountUnlocker)
        .build()
        .await;

    let ft1 = TokenId::Nep141(env.ft1.clone());
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    // lock
    {
        assert!(
            env.defuse
                .force_lock_account(env.user2.id())
                .await
                .expect("unable to lock an account")
        );
        assert!(
            env.defuse.is_account_locked(env.user2.id()).await.unwrap(),
            "account wasn't locked"
        );
        assert!(
            !env.defuse
                .force_lock_account(env.user2.id())
                .await
                .expect("unable to lock already locked account"),
            "second attempt to lock an account should not fail, but return `false`"
        );

        env.defuse_ft_deposit_to(&env.ft1, 250, env.user2.id())
            .await
            .expect("deposits should be allowed for locked account");

        env.user1
            .mt_transfer(
                env.defuse.id(),
                env.user2.id(),
                &ft1.to_string(),
                300,
                None,
                None,
            )
            .await
            .expect("locked accounts should be allowed to accept incoming transfers");

        env.defuse
            .execute_intents([env.user1.sign_defuse_message(
                env.defuse.id(),
                rng.random(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [Transfer {
                        receiver_id: env.user2.id().clone(),
                        tokens: TokenAmounts::default().with_add(ft1.clone(), 100).unwrap(),
                        memo: None,
                    }
                    .into()]
                    .into(),
                },
            )])
            .await
            .expect("locked accounts should be allowed to accept incoming transfers");

        assert_eq!(
            env.defuse
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep141(env.ft1.clone()).to_string()
                )
                .await
                .unwrap(),
            600,
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(
                    env.user2.id(),
                    &TokenId::Nep141(env.ft1.clone()).to_string()
                )
                .await
                .unwrap(),
            650, // 250 + 300 + 100
            "locked account balance mismatch"
        );

        env.user2
            .mt_transfer(
                env.defuse.id(),
                env.user3.id(),
                &ft1.to_string(),
                100,
                None,
                None,
            )
            .await
            .expect_err("locked accounts should not be able to make outgoing transfers");

        env.defuse
            .execute_intents([env.user2.sign_defuse_message(
                env.defuse.id(),
                rng.random(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [Transfer {
                        receiver_id: env.user3.id().clone(),
                        tokens: TokenAmounts::default().with_add(ft1.clone(), 200).unwrap(),
                        memo: None,
                    }
                    .into()]
                    .into(),
                },
            )])
            .await
            .expect_err("locked accounts should not be able to make outgoing transfers");

        assert_eq!(
            env.defuse
                .mt_balance_of(
                    env.user2.id(),
                    &TokenId::Nep141(env.ft1.clone()).to_string()
                )
                .await
                .unwrap(),
            650, // 250 + 300 + 100
            "balance of locked account should stay the same"
        );

        env.user2
            .add_public_key(env.defuse.id(), PublicKey::Ed25519(rng.random()))
            .await
            .expect_err("locked accounts should not be able to add new public keys");
        env.user2
            .remove_public_key(
                env.defuse.id(),
                env.user1
                    .secret_key()
                    .public_key()
                    .to_string()
                    .parse()
                    .unwrap(),
            )
            .await
            .expect_err("locked accounts should not be able to remove public keys");
    }

    // unlock
    {
        assert!(
            env.defuse
                .force_unlock_account(env.user2.id())
                .await
                .expect("unable to unlock a locked account")
        );
        assert!(
            !env.defuse.is_account_locked(env.user2.id()).await.unwrap(),
            "account wasn't unlocked"
        );
        assert!(
            !env.defuse
                .force_unlock_account(env.user2.id())
                .await
                .expect("unable to unlock already unlocked account"),
            "second attempt to unlock already unlocked account should not fail, but return `false`"
        );

        env.user2
            .mt_transfer(
                env.defuse.id(),
                env.user3.id(),
                &ft1.to_string(),
                100,
                None,
                None,
            )
            .await
            .expect("unlocked account should be allowed to transfer");

        env.defuse
            .execute_intents([env.user2.sign_defuse_message(
                env.defuse.id(),
                rng.random(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [Transfer {
                        receiver_id: env.user3.id().clone(),
                        tokens: TokenAmounts::default().with_add(ft1.clone(), 200).unwrap(),
                        memo: None,
                    }
                    .into()]
                    .into(),
                },
            )])
            .await
            .expect("unlocked account should be allowed to transfer");

        assert_eq!(
            env.defuse
                .mt_balance_of(
                    env.user2.id(),
                    &TokenId::Nep141(env.ft1.clone()).to_string()
                )
                .await
                .unwrap(),
            350, // 250 + 300 + 100 - 100 - 200
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(
                    env.user3.id(),
                    &TokenId::Nep141(env.ft1.clone()).to_string()
                )
                .await
                .unwrap(),
            300, // 100 + 200
        );
    }
}
