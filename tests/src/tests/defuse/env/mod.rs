#![allow(dead_code)]

mod builder;
mod state;
mod storage;

use super::{DefuseExt, accounts::AccountManagerExt};
use crate::{
    tests::{
        defuse::{
            env::{builder::EnvBuilder, storage::StorageMigration},
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
use futures::future::try_join_all;
use near_sdk::{AccountId, NearToken};
use near_workspaces::{
    Account, Contract, Network, Worker,
    operations::Function,
    types::{PublicKey, SecretKey},
};
use serde_json::json;
use std::{ops::Deref, sync::LazyLock};
use tokio::sync::Mutex;

pub use state::PersistentState;

pub static POA_TOKEN_WASM_NO_REGISTRATION: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/poa-token-no-registration/defuse_poa_token"));

pub struct Env {
    sandbox: Sandbox,

    pub wnear: Contract,

    pub defuse: Contract,

    pub poa_factory: Contract,

    pub disable_ft_storage_deposit: bool,
    pub disable_registration: bool,

    // Persistent state generated in case of migration tests
    // used to fetch existing accounts
    pub persistent_state: Option<PersistentState>,
    pub current_user_index: Mutex<usize>,
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

    pub async fn create_named_token(&self, name: &str) -> AccountId {
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

    pub async fn create_token(&self) -> AccountId {
        let account_id = generate_random_account_id(self.poa_factory.id());

        self.create_named_token(self.poa_factory.subaccount_name(&account_id).as_str())
            .await
    }

    pub async fn create_named_user(&self, name: &str) -> Result<Account> {
        let account = self.sandbox.create_account(name).await?;
        let pubkey = get_account_public_key(&account);

        if !self.defuse.has_public_key(account.id(), &pubkey).await? {
            account.add_public_key(self.defuse.id(), pubkey).await?;
        }

        Ok(account)
    }

    // Fetches user from persistent state or creates a new random one
    // in case if all users from persistent state are already used
    pub async fn create_user(&self) -> Account {
        let account_id = self.get_next_account_id().await;
        let root = self.sandbox.root_account();

        self.create_named_user(&root.subaccount_name(&account_id))
            .await
            .expect("Failed to create user account")
    }

    async fn get_next_account_id(&self) -> AccountId {
        if let Some(state) = self.persistent_state.as_ref() {
            let mut index = self.current_user_index.lock().await;

            if let Some(account_id) = state.accounts.keys().nth(*index) {
                *index += 1;

                return account_id.clone();
            }
        }

        generate_random_account_id(self.sandbox.root_account().id())
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
    pub async fn ft_storage_deposit_for_accounts(
        &self,
        accounts: impl IntoIterator<Item = &AccountId>,
        tokens: impl IntoIterator<Item = &AccountId>,
    ) {
        if self.disable_ft_storage_deposit {
            return;
        }

        let root = self.sandbox.root_account();
        let mut all_accounts: Vec<&AccountId> = accounts.into_iter().collect();

        all_accounts.push(self.defuse.id());
        all_accounts.push(root.id());

        // deposit WNEAR storage
        self.ft_deposit_for_accounts(&self.wnear.id(), all_accounts.clone())
            .await
            .expect("Failed to deposit Wnear storage");

        // deposit ALL tokens storage
        for token in tokens {
            self.ft_deposit_for_accounts(token, all_accounts.clone())
                .await
                .expect("Failed to deposit FT storage");

            // Deposit FTs to root for transfers to users
            self.ft_deposit_to_root(token)
                .await
                .expect("Failed to deposit FT storage to root");
        }
    }

    async fn ft_deposit_for_accounts(
        &self,
        token: &AccountId,
        accounts: impl IntoIterator<Item = &AccountId>,
    ) -> Result<()> {
        try_join_all(
            accounts
                .into_iter()
                .map(|acc| self.poa_factory.ft_storage_deposit(token, Some(acc))),
        )
        .await?;

        Ok(())
    }

    async fn ft_deposit_to_root(&self, token: &AccountId) -> Result<()> {
        self.poa_factory_ft_deposit(
            self.poa_factory.id(),
            &self.poa_ft_name(token),
            self.sandbox.root_account().id(),
            1_000_000_000,
            None,
            None,
        )
        .await
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

fn generate_random_account_id(parent_id: &AccountId) -> AccountId {
    let mut rng = make_true_rng();
    let bytes = rng.random::<[u8; 64]>();
    let u = &mut Unstructured::new(&bytes);

    ArbitraryNamedAccountId::arbitrary_subaccount(u, Some(parent_id))
        .expect("Failed to generate account ID")
}
