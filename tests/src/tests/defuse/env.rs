#![allow(dead_code)]

use std::ops::Deref;

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
use near_workspaces::{Account, Contract};

use crate::{
    tests::poa::factory::PoAFactoryExt,
    utils::{Sandbox, ft::FtExt, wnear::WNearExt},
};

use super::{DefuseExt, accounts::AccountManagerExt, tokens::nep141::DefuseFtReceiver};

pub struct Env {
    sandbox: Sandbox,

    pub user1: Account,
    pub user2: Account,
    pub user3: Account,

    pub wnear: Contract,

    pub defuse: Contract,

    pub poa_factory: Contract,

    pub ft1: AccountId,
    pub ft2: AccountId,
    pub ft3: AccountId,
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

    pub async fn defuse_ft_mint(
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

    pub fn poa_ft1_name(&self) -> &str {
        self.ft1
            .as_str()
            .strip_suffix(&format!(".{}", self.poa_factory.id()))
            .unwrap()
    }

    pub fn poa_ft2_name(&self) -> &str {
        self.ft2
            .as_str()
            .strip_suffix(&format!(".{}", self.poa_factory.id()))
            .unwrap()
    }

    pub fn poa_ft3_name(&self) -> &str {
        self.ft3
            .as_str()
            .strip_suffix(&format!(".{}", self.poa_factory.id()))
            .unwrap()
    }
}

impl Deref for Env {
    type Target = Account;

    fn deref(&self) -> &Self::Target {
        self.sandbox.root_account()
    }
}

#[derive(Debug, Default)]
pub struct EnvBuilder {
    fee: Pips,
    fee_collector: Option<AccountId>,

    // roles
    roles: RolesConfig,
    self_as_super_admin: bool,
    deployer_as_super_admin: bool,
}

impl EnvBuilder {
    pub fn fee(mut self, fee: Pips) -> Self {
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

    pub fn self_as_super_admin(mut self) -> Self {
        self.self_as_super_admin = true;
        self
    }

    pub fn deployer_as_super_admin(mut self) -> Self {
        self.deployer_as_super_admin = true;
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

    // pub fn staging_duration(mut self, staging_duration: Duration) -> Self {
    //     self.staging_duration = Some(staging_duration);
    //     self
    // }

    pub async fn build(mut self) -> Env {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();

        let poa_factory = root
            .deploy_poa_factory(
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
            .unwrap();

        let wnear = sandbox.deploy_wrap_near("wnear").await.unwrap();

        if self.self_as_super_admin {
            self.roles
                .super_admins
                .insert(format!("defuse.{}", root.id()).parse().unwrap());
        }
        if self.deployer_as_super_admin {
            self.roles.super_admins.insert(root.id().clone());
        }

        let s = Env {
            user1: sandbox.create_account("user1").await,
            user2: sandbox.create_account("user2").await,
            user3: sandbox.create_account("user3").await,
            defuse: root
                .deploy_defuse(
                    "defuse",
                    DefuseConfig {
                        wnear_id: wnear.id().clone(),
                        fees: FeesConfig {
                            fee: self.fee,
                            fee_collector: self.fee_collector.unwrap_or(root.id().clone()),
                        },
                        roles: self.roles,
                    },
                )
                .await
                .unwrap(),
            wnear,
            ft1: root
                .poa_factory_deploy_token(poa_factory.id(), "ft1", None)
                .await
                .unwrap(),
            ft2: root
                .poa_factory_deploy_token(poa_factory.id(), "ft2", None)
                .await
                .unwrap(),
            ft3: root
                .poa_factory_deploy_token(poa_factory.id(), "ft3", None)
                .await
                .unwrap(),
            poa_factory,
            sandbox,
        };

        s.ft_storage_deposit(
            s.wnear.id(),
            &[
                s.user1.id(),
                s.user2.id(),
                s.user3.id(),
                s.defuse.id(),
                root.id(),
            ],
        )
        .await
        .unwrap();
        s.near_deposit(s.wnear.id(), NearToken::from_near(100))
            .await
            .unwrap();

        s.ft_storage_deposit(
            &s.ft1,
            &[
                s.user1.id(),
                s.user2.id(),
                s.user3.id(),
                s.defuse.id(),
                root.id(),
            ],
        )
        .await
        .unwrap();
        s.ft_storage_deposit(
            &s.ft2,
            &[
                s.user1.id(),
                s.user2.id(),
                s.user3.id(),
                s.defuse.id(),
                root.id(),
            ],
        )
        .await
        .unwrap();
        s.ft_storage_deposit(
            &s.ft3,
            &[
                s.user1.id(),
                s.user2.id(),
                s.user3.id(),
                s.defuse.id(),
                root.id(),
            ],
        )
        .await
        .unwrap();
        for token in ["ft1", "ft2", "ft3"] {
            s.poa_factory_ft_deposit(
                s.poa_factory.id(),
                token,
                root.id(),
                1_000_000_000,
                None,
                None,
            )
            .await
            .unwrap();
        }

        // NOTE: near_workspaces uses the same signer all subaccounts
        s.user1
            .add_public_key(
                s.defuse.id(),
                // HACK: near_worspaces does not expose near_crypto API
                s.user1
                    .secret_key()
                    .public_key()
                    .to_string()
                    .parse()
                    .unwrap(),
            )
            .await
            .unwrap();
        s.user2
            .add_public_key(
                s.defuse.id(),
                s.user2
                    .secret_key()
                    .public_key()
                    .to_string()
                    .parse()
                    .unwrap(),
            )
            .await
            .unwrap();
        s.user3
            .add_public_key(
                s.defuse.id(),
                s.user3
                    .secret_key()
                    .public_key()
                    .to_string()
                    .parse()
                    .unwrap(),
            )
            .await
            .unwrap();

        s
    }
}
