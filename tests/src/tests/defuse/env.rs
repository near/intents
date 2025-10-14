#![allow(dead_code)]

use super::{DefuseExt, accounts::AccountManagerExt};
use crate::{
    tests::{defuse::tokens::nep141::traits::DefuseFtReceiver, poa::factory::PoAFactoryExt},
    utils::{Sandbox, ft::FtExt, read_wasm, wnear::WNearExt},
};
use anyhow::anyhow;
use defuse::{
    contract::{
        Role,
        config::{DefuseConfig, RolesConfig},
    },
    core::fees::{FeesConfig, Pips},
    tokens::DepositMessage,
};
use defuse_poa_factory::contract::Role as POAFactoryRole;
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

        // TODO: remove this?
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

    pub async fn create_user(&self, name: &str) -> Account {
        let account = self.sandbox.create_account(name).await;

        account
            .add_public_key(self.defuse.id(), get_defuse_public_key(&account))
            .await
            .unwrap();

        account
    }

    // if no tokens provided - only wnear storage deposit will be done
    pub async fn deposit_to_users(&self, mut accounts: Vec<&AccountId>, tokens: &[&AccountId]) {
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

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct EnvBuilder {
    fee: Pips,
    fee_collector: Option<AccountId>,

    // roles
    roles: RolesConfig,
    self_as_super_admin: bool,
    deployer_as_super_admin: bool,
    disable_ft_storage_deposit: bool,
    disable_registration: bool,
}

impl EnvBuilder {
    pub const fn fee(mut self, fee: Pips) -> Self {
        self.fee = fee;
        self
    }

    pub fn fee_collector(mut self, fee_collector: AccountId) -> Self {
        self.fee_collector = Some(fee_collector);
        self
    }

    pub fn super_admin(mut self, super_admin: AccountId) -> Self {
        self.roles.super_admins.insert(super_admin);
        self
    }

    pub const fn self_as_super_admin(mut self) -> Self {
        self.self_as_super_admin = true;
        self
    }

    pub const fn deployer_as_super_admin(mut self) -> Self {
        self.deployer_as_super_admin = true;
        self
    }

    pub const fn disable_ft_storage_deposit(mut self) -> Self {
        self.disable_ft_storage_deposit = true;
        self
    }

    pub fn admin(mut self, role: Role, admin: AccountId) -> Self {
        self.roles.admins.entry(role).or_default().insert(admin);
        self
    }

    pub fn grantee(mut self, role: Role, grantee: AccountId) -> Self {
        self.roles.grantees.entry(role).or_default().insert(grantee);
        self
    }

    pub const fn no_registration(mut self, no_reg_value: bool) -> Self {
        self.disable_registration = no_reg_value;
        self
    }

    async fn deploy_defuse(&self, root: &Account, wnear: &Contract) -> Contract {
        let id = "defuse";
        let cfg = DefuseConfig {
            wnear_id: wnear.id().clone(),
            fees: FeesConfig {
                fee: self.fee,
                fee_collector: self
                    .fee_collector
                    .as_ref()
                    .unwrap_or_else(|| root.id())
                    .clone(),
            },
            roles: self.roles.clone(),
        };

        root.deploy_defuse(id, cfg).await.unwrap()
    }

    async fn deploy_legacy_and_migrate(&self, root: &Account, wnear: &Contract) -> Contract {
        let contract = self.deploy_defuse(root, &wnear).await;

        contract.upgrade_defuse().await.unwrap();

        contract
    }

    fn grant_roles(&mut self, root: &Account) {
        if self.self_as_super_admin {
            self.roles
                .super_admins
                .insert(format!("defuse.{}", root.id()).parse().unwrap());
        }

        if self.deployer_as_super_admin {
            self.roles.super_admins.insert(root.id().clone());
        }
    }
    pub async fn build(mut self) -> Env {
        // TODO: remove dublication
        let migrate_from_legacy = std::env::var("MIGRATE_FROM_LEGACY").is_ok_and(|v| v != "0");

        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();

        let poa_factory = deploy_poa_factory(&root).await;
        let wnear = sandbox.deploy_wrap_near("wnear").await.unwrap();

        self.grant_roles(&root);

        let defuse = if migrate_from_legacy {
            self.deploy_legacy_and_migrate(&root, &wnear).await
        } else {
            self.deploy_defuse(&root, &wnear).await
        };

        let env = Env {
            defuse,
            wnear,
            poa_factory: poa_factory.clone(),
            sandbox,
            disable_ft_storage_deposit: self.disable_ft_storage_deposit,
            disable_registration: self.disable_registration,
        };

        env.near_deposit(env.wnear.id(), NearToken::from_near(100))
            .await
            .unwrap();

        env
    }
}

async fn deploy_poa_factory(root: &Account) -> Contract {
    root.deploy_poa_factory(
        "poa-factory",
        [root.id().clone()],
        [
            (POAFactoryRole::TokenDeployer, [root.id().clone()]),
            (POAFactoryRole::TokenDepositer, [root.id().clone()]),
        ],
        [
            (POAFactoryRole::TokenDeployer, [root.id().clone()]),
            (POAFactoryRole::TokenDepositer, [root.id().clone()]),
        ],
    )
    .await
    .unwrap()
}

fn get_defuse_public_key(account: &Account) -> defuse::core::crypto::PublicKey {
    account
        .secret_key()
        .public_key()
        .to_string()
        .parse()
        .unwrap()
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
