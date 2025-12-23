#![allow(dead_code)]

mod builder;
mod state;
mod storage;

use super::DefuseSignerExt;
use crate::tests::defuse::env::builder::EnvBuilder;
use anyhow::{Ok, Result, anyhow};
use arbitrary::Unstructured;
use defuse::sandbox_ext::{
    account_manager::{AccountManagerExt, AccountViewExt},
    tokens::nep141::DefuseFtDepositor,
};
use defuse::{
    core::{Deadline, ExpirableNonce, Nonce, Salt, SaltedNonce, VersionedNonce},
    tokens::{DepositAction, DepositMessage},
};
use defuse_poa_factory::sandbox_ext::PoAFactoryExt;
use defuse_randomness::{Rng, make_true_rng};
use defuse_sandbox::extensions::storage_management::StorageManagementExt;
use defuse_sandbox::tx::FnCallBuilder;
use defuse_sandbox::{Account, Sandbox, SigningAccount, read_wasm};
use defuse_test_utils::random::{Seed, rng};
use futures::future::try_join_all;
use impl_tools::autoimpl;
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::AccountIdRef;
use near_sdk::{AccountId, NearToken, account_id::arbitrary::ArbitraryNamedAccountId, env::sha256};

use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/multi-token-receiver-stub/multi_token_receiver_stub"));

const TOKEN_STORAGE_DEPOSIT: NearToken = NearToken::from_near(1);
const INITIAL_USER_BALANCE: NearToken = NearToken::from_near(10);

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    sandbox: Sandbox,

    pub wnear: Account,

    pub defuse: Account,

    pub poa_factory: Account,

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

    pub fn root(&self) -> &SigningAccount {
        self.sandbox.root()
    }

    pub async fn get_unique_nonce(&self, deadline: Option<Deadline>) -> anyhow::Result<Nonce> {
        let root = self.root();
        root.unique_nonce(&self.defuse, deadline).await
    }

    pub async fn defuse_ft_deposit_to(
        &self,
        token_id: &AccountIdRef,
        amount: u128,
        to: &AccountIdRef,
        action: impl Into<Option<DepositAction>>,
    ) -> anyhow::Result<()> {
        if self
            .defuse_ft_deposit(
                self.defuse.id(),
                token_id,
                amount,
                DepositMessage::new(to.into()).with_action(action),
            )
            .await?
            != amount
        {
            return Err(anyhow!("refunded"));
        }
        Ok(())
    }

    pub async fn create_named_token(&self, name: impl AsRef<str>) -> Account {
        let root = self.root();

        root.poa_factory_deploy_token(self.poa_factory.id(), name, None)
            .await
            .unwrap()
    }

    pub async fn create_token(&self) -> Account {
        let account_id = generate_random_account_id(self.poa_factory.id())
            .expect("Failed to generate random account ID");
        let name = account_id
            .as_str()
            .trim_end_matches(&format!(".{}", self.poa_factory.id()));

        self.create_named_token(name).await
    }

    pub async fn create_ft_token_with_initial_balances(
        &self,
        balances: impl IntoIterator<Item = (AccountId, u128)>,
    ) -> anyhow::Result<Account> {
        let account_id = generate_random_account_id(self.poa_factory.id())
            .expect("Failed to generate random account ID");
        let name = account_id
            .as_str()
            .trim_end_matches(&format!(".{}", self.poa_factory.id()));

        let token = self.create_named_token(name).await;
        let balances = balances.into_iter().collect::<Vec<_>>();

        // First: storage deposits concurrently
        self.ft_storage_deposit_for_accounts(token.id(), balances.iter().map(|(user, _)| user))
            .await?;

        // Then: token minting concurrently
        let token_name = self.poa_ft_name(token.id());
        try_join_all(balances.iter().filter_map(|(user, amount)| {
            (*amount > 0).then(|| {
                self.root().poa_factory_ft_deposit(
                    self.poa_factory.id(),
                    &token_name,
                    user,
                    *amount,
                    None,
                    None,
                )
            })
        }))
        .await?;

        Ok(token)
    }

    pub async fn create_mt_token_with_initial_balances(
        &self,
        balances: impl IntoIterator<Item = (AccountId, u128)>,
    ) -> anyhow::Result<Account> {
        let token = self.create_token().await;
        let balances = balances.into_iter().collect::<Vec<_>>();

        // Storage deposit only for defuse and root
        self.ft_storage_deposit_for_accounts(token.id(), [self.defuse.id(), self.root().id()])
            .await?;

        // Mint tokens to root (so root can transfer to defuse)
        self.ft_deposit_to_root(token.id()).await?;

        // Deposit to defuse for each user concurrently
        try_join_all(balances.iter().filter_map(|(user, amount)| {
            (*amount > 0).then(|| self.defuse_ft_deposit_to(token.id(), *amount, user, None))
        }))
        .await?;

        Ok(token)
    }

    pub async fn create_named_user(&self, name: &str) -> SigningAccount {
        let account = self
            .generate_subaccount(name, INITIAL_USER_BALANCE)
            .await
            .expect("Failed to create user");

        let pubkey = account.signer().get_public_key().await.unwrap().into();

        if !self
            .defuse
            .has_public_key(account.id(), &pubkey)
            .await
            .expect("Failed to check publick key")
        {
            account
                .add_public_key(self.defuse.id(), &pubkey)
                .await
                .expect("Failed to add pubkey");
        }

        account
    }

    pub async fn create_user(&self) -> SigningAccount {
        let account_id = self
            .get_next_account_id()
            .expect("Failed to generate next account id");

        println!("Creating user account: {}", &account_id);
        let name = account_id
            .as_str()
            .trim_end_matches(&format!(".{}", self.root().id()));

        self.create_named_user(name).await
    }

    // Randomly derives account ID from seed and unique index
    // (to match existing accounts in migration tests)
    // Or create new arbitrary account id
    fn get_next_account_id(&self) -> Result<AccountId> {
        let mut rand = make_true_rng();
        let root = self.root();

        // NOTE: every second account is legacy
        if rand.random() {
            let index = self.next_user_index.fetch_add(1, Ordering::SeqCst);
            Ok(generate_legacy_user_account_id(root, index, self.seed)
                .expect("Failed to generate account ID"))
        } else {
            generate_random_account_id(root.id())
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

        let root = self.root();
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
        token: &AccountIdRef,
        accounts: impl IntoIterator<Item = &AccountId>,
    ) -> Result<()> {
        try_join_all(accounts.into_iter().map(|acc| {
            self.sandbox
                .root()
                .storage_deposit(token, acc.as_ref(), TOKEN_STORAGE_DEPOSIT)
        }))
        .await?;

        Ok(())
    }

    async fn ft_deposit_to_root(&self, token: &AccountIdRef) -> Result<()> {
        self.poa_factory_ft_deposit(
            self.poa_factory.id(),
            &self.poa_ft_name(token),
            self.root().id(),
            1_000_000_000,
            None,
            None,
        )
        .await
    }

    pub fn poa_ft_name(&self, ft: &AccountIdRef) -> String {
        ft.as_str()
            .trim_end_matches(&format!(".{}", self.poa_factory.id()))
            .to_string()
    }

    pub async fn fund_account_with_near(
        &self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<()> {
        self.sandbox
            .root()
            .tx(account_id)
            .transfer(amount)
            .await
            .map(|_| ())
    }

    pub async fn deploy_mt_receiver_stub(&self) -> SigningAccount {
        self.sandbox()
            .root()
            .deploy_sub_contract(
                "mt_receiver_stub",
                NearToken::from_near(100),
                MT_RECEIVER_STUB_WASM.to_vec(),
                None::<FnCallBuilder>,
            )
            .await
            .unwrap()
    }

    pub async fn near_balance(&self, account: &Account) -> NearToken {
        account.view().await.unwrap().amount
    }

    pub const fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn sandbox_mut(&mut self) -> &mut Sandbox {
        &mut self.sandbox
    }
}

#[derive(Debug, Clone)]
pub struct TransferCallExpectation {
    pub mode: MTReceiverMode,
    pub intent_transfer_amount: Option<u128>,
    pub expected_sender_balance: u128,
    pub expected_receiver_balance: u128,
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

fn generate_random_account_id(parent_id: &AccountId) -> Result<AccountId> {
    let mut rng = make_true_rng();
    ArbitraryNamedAccountId::arbitrary_subaccount(
        &mut Unstructured::new(&rng.random::<[u8; 64]>()),
        Some(parent_id),
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate account ID : {e}"))
}

fn generate_legacy_user_account_id(
    parent_id: &Account,
    index: usize,
    seed: Seed,
) -> Result<AccountId> {
    let bytes = sha256((seed.as_u64() + u64::try_from(index)?).to_be_bytes())[..8]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to create new account seed"))?;
    let seed = Seed::from_u64(u64::from_be_bytes(bytes));
    let mut rng = rng(seed);
    ArbitraryNamedAccountId::arbitrary_subaccount(
        &mut Unstructured::new(&rng.random::<[u8; 64]>()),
        Some(parent_id.id()),
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate account ID : {e}"))
}
