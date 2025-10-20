#![allow(dead_code)]

mod builder;
mod state;
mod storage;

use super::{DefuseExt, accounts::AccountManagerExt};
use crate::{
    tests::{
        defuse::{
            env::{builder::EnvBuilder, state::PersistentState, storage::StorageMigration},
            tokens::nep141::traits::DefuseFtReceiver,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{ParentAccount, Sandbox, acl::AclExt, ft::FtExt, read_wasm},
};
use anyhow::{Ok, Result, anyhow};
use arbitrary::Unstructured;
use defuse::{
    contract::Role,
    core::{Deadline, ExpirableNonce, Nonce, Salt, SaltedNonce, VersionedNonce},
    tokens::DepositMessage,
};
use defuse_near_utils::arbitrary::ArbitraryNamedAccountId;
use defuse_randomness::{Rng, make_true_rng};
use near_sdk::{AccountId, NearToken};
use near_workspaces::{
    Account, Contract, Network, Worker,
    operations::Function,
    types::{PublicKey, SecretKey},
};
use serde_json::json;
use std::{ops::Deref, sync::LazyLock};

pub static POA_TOKEN_WASM_NO_REGISTRATION: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res", "poa-token-no-registration/defuse_poa_token"));

pub struct Env {
    sandbox: Sandbox,

    pub wnear: Contract,

    pub defuse: Contract,

    pub poa_factory: Contract,

    pub disable_ft_storage_deposit: bool,
    pub disable_registration: bool,

    pub persistent_state: Option<PersistentState>,
    pub current_user_index: usize,
}

impl Env {
    pub fn builder() -> EnvBuilder {
        EnvBuilder::default()
    }

    pub async fn new() -> Self {
        Self::builder().build().await
    }

    pub async fn ft_storage_deposit(
        &self,
        token: &AccountId,
        accounts: &[&AccountId],
    ) -> anyhow::Result<()> {
        self.sandbox
            .root_account()
            .ft_storage_deposit_many(token, accounts)
            .await
    }

    pub async fn defuse_ft_deposit_to(
        &self,
        token_id: &AccountId,
        amount: u128,
        to: &AccountId,
    ) -> anyhow::Result<()> {
        if self
            .defuse_ft_deposit(
                self.defuse.id(),
                token_id,
                amount,
                DepositMessage::new(to.clone()),
            )
            .await?
            != amount
        {
            return Err(anyhow!("refunded"));
        }
        Ok(())
    }

    pub async fn create_token(&self, name: &str) -> AccountId {
        let root = self.sandbox.root_account();

        let ft = root
            .poa_factory_deploy_token(self.poa_factory.id(), name, None)
            .await
            .unwrap();

        if self.disable_registration {
            let root_secret_key = root.secret_key();
            let root_access_key = root_secret_key.public_key();

            let worker = self.sandbox.worker().clone();

            deploy_token_without_registration(
                self,
                &ft,
                &root_access_key,
                root_secret_key,
                worker.clone(),
            )
            .await;
        }

        ft
    }

    pub async fn create_named_user(&self, name: &str) -> Result<Account> {
        let account = self.sandbox.create_account(name).await?;
        let pubkey = get_account_public_key(&account);

        if !self.defuse.has_public_key(account.id(), &pubkey).await? {
            account.add_public_key(self.defuse.id(), pubkey).await?
        }

        Ok(account)
    }

    pub async fn get_or_create_user(&mut self) -> Account {
        let account_id = self.get_next_account_id();
        let root = self.sandbox.root_account();

        self.create_named_user(&root.subaccount_name(&account_id))
            .await
            .expect("Failed to create user account")
    }

    fn get_next_account_id(&mut self) -> AccountId {
        if let Some(state) = &self.persistent_state {
            let account_id = state
                .accounts
                .keys()
                .nth(self.current_user_index)
                .expect("No more accounts in persistent state")
                .clone();

            self.current_user_index += 1;

            account_id
        } else {
            self.generate_random_account_id()
        }
    }

    fn generate_random_account_id(&self) -> AccountId {
        let parent_id = self.sandbox.root_account().id();
        let mut rng = make_true_rng();
        let bytes = rng.random::<[u8; 64]>();
        let u = &mut Unstructured::new(&bytes);

        ArbitraryNamedAccountId::arbitrary_subaccount(u, Some(parent_id))
            .expect("Failed to generate account ID")
    }

    pub async fn upgrade_legacy(&mut self) {
        self.generate_storage_data().await;

        self.acl_grant_role(
            self.defuse.id(),
            Role::Upgrader,
            self.sandbox.root_account().id(),
        )
        .await
        .expect("Failed to grant upgrader role");

        self.upgrade_defuse(self.defuse.id())
            .await
            .expect("Failed to upgrade defuse");

        self.verify_storage_consistency().await;
    }

    // if no tokens provided - only wnear storage deposit will be done
    pub async fn ft_storage_deposit_for_users(
        &self,
        mut accounts: Vec<&AccountId>,
        tokens: &[&AccountId],
    ) {
        let root = self.sandbox.root_account();

        accounts.push(self.defuse.id());
        accounts.push(root.id());

        if !self.disable_ft_storage_deposit {
            // Wnear storage deposit
            root.ft_storage_deposit_many(self.wnear.id(), &accounts)
                .await
                .unwrap();

            // FT storage deposit
            for token in tokens {
                self.poa_factory
                    .ft_storage_deposit_many(token, &accounts)
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn ft_deposit_to_root(&self, tokens: &[&AccountId]) {
        let root = self.sandbox.root_account();

        for ft in tokens {
            self.poa_factory_ft_deposit(
                self.poa_factory.id(),
                &self.poa_ft_name(ft),
                root.id(),
                1_000_000_000,
                None,
                None,
            )
            .await
            .unwrap();
        }
    }

    pub fn poa_ft_name(&self, ft: &AccountId) -> String {
        ft.as_str()
            .strip_suffix(&format!(".{}", self.poa_factory.id()))
            .unwrap()
            .to_string()
    }

    pub async fn fund_account_with_near(&self, account_id: &AccountId, amount: NearToken) {
        self.sandbox
            .root_account()
            .transfer_near(account_id, amount)
            .await
            .unwrap()
            .unwrap();
    }

    pub async fn near_balance(&self, account_id: &AccountId) -> NearToken {
        self.sandbox
            .worker()
            .view_account(account_id)
            .await
            .unwrap()
            .balance
    }

    pub const fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn sandbox_mut(&mut self) -> &mut Sandbox {
        &mut self.sandbox
    }
}

impl Deref for Env {
    type Target = Account;

    fn deref(&self) -> &Self::Target {
        self.sandbox.root_account()
    }
}

async fn deploy_token_without_registration<N: Network + 'static>(
    env_result: &Env,
    ft: &AccountId,
    root_access_key: &PublicKey,
    root_secret_key: &SecretKey,
    worker: Worker<N>,
) {
    env_result
        .poa_factory
        .as_account()
        .batch(ft)
        .call(
            Function::new("add_full_access_key")
                .args_json(json!({"public_key": root_access_key}))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    Contract::from_secret_key(ft.clone(), root_secret_key.clone(), &worker)
        .batch()
        .deploy(&POA_TOKEN_WASM_NO_REGISTRATION)
        .delete_key(root_access_key.clone())
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();
}

pub fn get_account_public_key(account: &Account) -> defuse::core::crypto::PublicKey {
    account
        .secret_key()
        .public_key()
        .to_string()
        .parse()
        .unwrap()
}

pub fn create_random_salted_nonce(salt: Salt, deadline: Deadline, mut rng: impl Rng) -> Nonce {
    VersionedNonce::V1(SaltedNonce::new(
        salt,
        ExpirableNonce {
            deadline,
            nonce: rng.random::<[u8; 15]>(),
        },
    ))
    .into()
}
