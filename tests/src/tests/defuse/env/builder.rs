use defuse_fees::Pips;
use defuse_sandbox::{
    account::Account,
    extensions::{
        defuse::{
            Defuse, DefuseClient,
            contract::{
                Role,
                config::{DefuseConfig, RolesConfig},
            },
            core::fees::FeesConfig,
        },
        poa::{PoaFactoryClient, PoaFactoryDeployerExt, contract::Role as POAFactoryRole},
        wnear::{WNearDeployerExt, WNearExt},
    },
    kit::{AccountId, AccountIdRef, Action, Final, FunctionCallAction, Gas, Near, NearToken},
};
use defuse_test_utils::wasms::{DEFUSE_WASM, POA_FACTORY_WASM, WNEAR_WASM};
use serde_json::json;

use crate::tests::defuse::env::Env;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct EnvBuilder {
    fee: Pips,
    fee_collector: Option<AccountId>,

    // roles
    roles: RolesConfig,
    self_as_super_admin: bool,
    deployer_as_super_admin: bool,
    disable_ft_storage_deposit: bool,

    defuse_name: String,
    defuse_wasm: Vec<u8>,

    poa_factory_name: String,
}

impl Default for EnvBuilder {
    fn default() -> Self {
        Self {
            fee: Pips::ZERO,
            fee_collector: None, // self
            roles: RolesConfig::default(),
            self_as_super_admin: false,
            deployer_as_super_admin: false,
            disable_ft_storage_deposit: false,
            defuse_name: "defuse".to_string(),
            defuse_wasm: DEFUSE_WASM.clone(),
            poa_factory_name: "poa-factory".to_string(),
        }
    }
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

    pub fn defuse_name(mut self, name: impl Into<String>) -> Self {
        self.defuse_name = name.into();
        self
    }

    pub fn poa_factory_name(mut self, name: impl Into<String>) -> Self {
        self.poa_factory_name = name.into();
        self
    }

    pub fn legacy(mut self) -> Self {
        use defuse_test_utils::wasms::DEFUSE_LEGACY_WASM;

        self.defuse_wasm = DEFUSE_LEGACY_WASM.clone();
        self
    }

    #[cfg(feature = "imt")]
    pub fn imt(mut self) -> Self {
        use defuse_test_utils::wasms::DEFUSE_FAR_WASM;

        self.defuse_wasm = DEFUSE_FAR_WASM.clone();
        self
    }

    async fn deploy_defuse(
        &self,
        name: &str,
        root: &Near,
        wnear: impl AsRef<AccountIdRef>,
    ) -> (DefuseClient, Near) {
        let cfg = DefuseConfig {
            wnear_id: wnear.as_ref().into(),
            fees: FeesConfig {
                fee: self.fee,
                fee_collector: self
                    .fee_collector
                    .as_ref()
                    .unwrap_or_else(|| root.account_id())
                    .clone(),
            },
            roles: self.roles.clone(),
        };

        let account = root
            .create_subaccount(name, NearToken::from_near(100))
            .await;

        account
            .deploy(self.defuse_wasm.clone())
            .add_action(Action::FunctionCall(FunctionCallAction {
                method_name: "new".to_string(),
                args: serde_json::to_vec(&json!({"config": cfg})).unwrap(),
                gas: Gas::from_tgas(30),
                deposit: NearToken::from_near(0),
            }))
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .unwrap();

        let client = root.contract::<Defuse>(account.account_id());
        (client, account)
    }

    fn grant_roles(&mut self, root: impl AsRef<AccountIdRef>) {
        if self.self_as_super_admin {
            self.roles
                .super_admins
                .insert(root.as_ref().sub_account(&self.defuse_name).unwrap());
        }

        if self.deployer_as_super_admin {
            self.roles.super_admins.insert(root.as_ref().into());
        }
    }

    pub async fn build(mut self, root: Near) -> Env {
        self.grant_roles(root.account_id());

        let (poa_factory, wnear) = futures::join!(
            self.deploy_poa_factory(&root),
            root.deploy_wrap_near("wnear", WNEAR_WASM.clone())
        );
        let (defuse, defuse_near) = self
            .deploy_defuse(&self.defuse_name, &root, wnear.contract_id())
            .await;

        let env = Env {
            defuse,
            defuse_near,
            wnear,
            poa_factory,
            root,
            disable_ft_storage_deposit: self.disable_ft_storage_deposit,
        };

        env.near_deposit(env.wnear.contract_id(), NearToken::from_near(100))
            .await
            .unwrap();

        env
    }

    async fn deploy_poa_factory(&self, root: &Near) -> PoaFactoryClient {
        let root_id = root.account_id();
        root.deploy_poa_factory(
            &self.poa_factory_name,
            [root.account_id().clone()],
            [
                (POAFactoryRole::TokenDeployer, [root_id.clone()]),
                (POAFactoryRole::TokenDepositer, [root_id.clone()]),
            ],
            [
                (POAFactoryRole::TokenDeployer, [root_id.clone()]),
                (POAFactoryRole::TokenDepositer, [root_id.clone()]),
            ],
            POA_FACTORY_WASM.clone(),
        )
        .await
    }
}
