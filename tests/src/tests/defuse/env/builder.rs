use super::DefuseExt;
use crate::{
    tests::{
        defuse::env::{Env, storage::StorageMigration},
        poa::factory::PoAFactoryExt,
    },
    utils::{Sandbox, acl::AclExt, wnear::WNearExt},
};
use defuse::{
    contract::{
        Role,
        config::{DefuseConfig, RolesConfig},
    },
    core::fees::{FeesConfig, Pips},
};
use defuse_poa_factory::contract::Role as POAFactoryRole;
use near_sdk::{AccountId, NearToken};
use near_workspaces::{Account, Contract};

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

    async fn deploy_defuse(&self, root: &Account, wnear: &Contract, legacy: bool) -> Contract {
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

        root.deploy_defuse(id, cfg, legacy).await.unwrap()
    }

    fn grant_roles(&mut self, root: &Account, deploy_legacy: bool) {
        if self.self_as_super_admin {
            self.roles
                .super_admins
                .insert(format!("defuse.{}", root.id()).parse().unwrap());
        }

        if self.deployer_as_super_admin || deploy_legacy {
            self.roles.super_admins.insert(root.id().clone());
        }
    }

    pub async fn build_env(&mut self, deploy_legacy: bool) -> Env {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();

        let poa_factory = deploy_poa_factory(&root).await;
        let wnear = sandbox.deploy_wrap_near("wnear").await.unwrap();

        self.grant_roles(&root, deploy_legacy);

        let defuse = self.deploy_defuse(&root, &wnear, deploy_legacy).await;

        let mut env = Env {
            defuse,
            wnear,
            poa_factory: poa_factory.clone(),
            sandbox,
            disable_ft_storage_deposit: self.disable_ft_storage_deposit,
            disable_registration: self.disable_registration,
            arbitrary_state: None,
        };

        if deploy_legacy {
            env.upgrade_legacy().await;
        }

        env.near_deposit(env.wnear.id(), NearToken::from_near(100))
            .await
            .unwrap();

        env
    }

    pub async fn build_without_migration(&mut self) -> Env {
        self.build_env(false).await
    }

    pub async fn build_with_migration(&mut self) -> Env {
        self.build_env(true).await
    }

    pub async fn build(&mut self) -> Env {
        let migrate_from_legacy = std::env::var("MIGRATE_FROM_LEGACY").is_ok_and(|v| v != "0");

        self.build_env(migrate_from_legacy).await
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
