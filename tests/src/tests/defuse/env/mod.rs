#![allow(dead_code)]

mod builder;
mod state;
mod storage;

use super::{DefuseExt, DefuseSignerExt, accounts::AccountManagerExt};
use crate::{
    tests::{
        defuse::{env::builder::EnvBuilder, tokens::nep141::traits::DefuseFtReceiver},
        poa::factory::PoAFactoryExt,
    },
    utils::{ParentAccount, Sandbox, ft::FtExt, read_wasm},
};
use anyhow::{Ok, Result, anyhow};
use arbitrary::Unstructured;
use defuse::{
    core::{Deadline, ExpirableNonce, Nonce, Salt, SaltedNonce, VersionedNonce},
    tokens::DepositMessage,
};
use defuse_near_utils::arbitrary::ArbitraryNamedAccountId;
use defuse_randomness::{Rng, make_true_rng};
use defuse_test_utils::random::{Seed, rng};
use futures::future::try_join_all;
use near_sdk::{AccountId, NearToken, env::sha256};
use near_workspaces::{
    Account, Contract, Network, Worker,
    operations::Function,
    types::{PublicKey, SecretKey},
};
use serde_json::json;
use std::{
    ops::Deref,
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

pub static POA_TOKEN_WASM_NO_REGISTRATION: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/poa-token-no-registration/defuse_poa_token"));

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/multi-token-receiver-stub/multi_token_receiver_stub"));

pub struct Env {
    sandbox: Sandbox,

    pub wnear: Contract,

    pub defuse: Contract,

    pub poa_factory: Contract,

    pub disable_ft_storage_deposit: bool,
    pub disable_registration: bool,

    // Persistent state generated in case of migration tests
    // used to fetch existing accounts
    pub next_user_index: AtomicUsize,
    pub seed: Seed,
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

    pub async fn get_unique_nonce(&self, deadline: Option<Deadline>) -> anyhow::Result<Nonce> {
        let root = self.sandbox.root_account();
        root.unique_nonce(self.defuse.id(), deadline).await
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
        let account_id = generate_random_account_id(self.poa_factory.id(), Some("token-"))
            .expect("Failed to generate random account ID");

        self.create_named_token(self.poa_factory.subaccount_name(&account_id).as_str())
            .await
    }

    pub async fn create_named_user(&self, name: &str) -> Account {
        let account = self
            .sandbox
            .create_account(name)
            .await
            .expect("Failed to create account");
        let pubkey = get_account_public_key(&account);

        if !self
            .defuse
            .has_public_key(account.id(), &pubkey)
            .await
            .expect("Failed to check publick key")
        {
            account
                .add_public_key(self.defuse.id(), pubkey)
                .await
                .expect("Failed to add pubkey");
        }

        account
    }

    pub async fn create_user(&self) -> Account {
        let account_id = self
            .get_next_account_id()
            .expect("Failed to generate next account id");
        let root = self.sandbox.root_account();

        self.create_named_user(&root.subaccount_name(&account_id))
            .await
    }

    // Randomly derives account ID from seed and unique index
    // (to match existing accounts in migration tests)
    // Or create new arbitrary account id
    fn get_next_account_id(&self) -> Result<AccountId> {
        let mut rand = make_true_rng();
        let root = self.sandbox.root_account();

        // NOTE: every second account is legacy
        if rand.random() {
            let index = self.next_user_index.fetch_add(1, Ordering::SeqCst);
            Ok(generate_legacy_user_account_id(root, index, self.seed)
                .expect("Failed to generate account ID"))
        } else {
            generate_random_account_id(root.id(), None)
        }
    }

    // if no tokens provided - only wnear storage deposit will be done
    pub async fn initial_ft_storage_deposit(
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
        self.ft_storage_deposit_for_accounts(self.wnear.id(), all_accounts.clone())
            .await
            .expect("Failed to deposit Wnear storage");

        // deposit ALL tokens storage
        for token in tokens {
            self.ft_storage_deposit_for_accounts(token, all_accounts.clone())
                .await
                .expect("Failed to deposit FT storage");

            // Deposit FTs to root for transfers to users
            self.ft_deposit_to_root(token)
                .await
                .expect("Failed to deposit FT storage to root");
        }
    }

    async fn ft_storage_deposit_for_accounts(
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

fn generate_random_account_id(parent_id: &AccountId, prefix: Option<&str>) -> Result<AccountId> {
    let mut rng = make_true_rng();
    ArbitraryNamedAccountId::arbitrary_subaccount(
        &mut Unstructured::new(&rng.random::<[u8; 64]>()),
        prefix,
        Some(parent_id),
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate account ID : {}", e))
}

fn generate_legacy_user_account_id(
    parent_id: &Account,
    index: usize,
    seed: Seed,
) -> Result<AccountId> {
    let bytes = sha256(&(seed.as_u64() + u64::try_from(index)?).to_be_bytes())[..8]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to create new account seed"))?;
    let seed = Seed::from_u64(u64::from_be_bytes(bytes));
    let mut rng = rng(seed);
    ArbitraryNamedAccountId::arbitrary_subaccount(
        &mut Unstructured::new(&rng.random::<[u8; 64]>()),
        Some(&format!("legacy-user{index}-")),
        Some(parent_id.id()),
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate account ID : {}", e))
}
