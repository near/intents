use defuse_fees::Pips;
use defuse_sandbox::{
    account::Account,
    extensions::{
        DEFAULT_GAS,
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
    kit::{Action, Final, FunctionCallAction, Near, NearToken},
    root,
};
use defuse_test_utils::{
    random::Seed,
    wasms::{DEFUSE_FAR_WASM, POA_FACTORY_WASM, WNEAR_WASM},
};
use near_sdk::{AccountId, AccountIdRef, serde_json::json};

use crate::tests::defuse::env::Env;

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

    defuse_wasm: Option<Vec<u8>>,
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

    pub fn defuse_wasm(mut self, wasm: Vec<u8>) -> Self {
        self.defuse_wasm = Some(wasm);
        self
    }

    async fn deploy_defuse(
        &self,
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

        let wasm = self
            .defuse_wasm
            .clone()
            .unwrap_or_else(|| DEFUSE_FAR_WASM.clone());

        let account = root
            .create_subaccount("defuse", NearToken::from_near(100))
            .await;

        account
            .deploy(wasm)
            .add_action(Action::FunctionCall(FunctionCallAction {
                method_name: "new".to_string(),
                args: json!({"config": cfg}).to_string().as_bytes().to_vec(),
                gas: DEFAULT_GAS,
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
                .insert(format!("defuse.{}", root.as_ref()).parse().unwrap());
        }

        if self.deployer_as_super_admin {
            self.roles.super_admins.insert(root.as_ref().into());
        }
    }

    pub async fn build(&mut self) -> Env {
        let root = root(NearToken::from_near(100_000)).await;

        self.grant_roles(root.account_id());

        let poa_factory = deploy_poa_factory(&root).await;
        let wnear = root.deploy_wrap_near("wnear", WNEAR_WASM.clone()).await;
        let (defuse, defuse_near) = self.deploy_defuse(&root, wnear.contract_id()).await;

        let env = Env {
            defuse: defuse.into(),
            defuse_near,
            wnear: wnear.into(),
            poa_factory: poa_factory.into(),
            root,
            disable_ft_storage_deposit: self.disable_ft_storage_deposit,
            disable_registration: self.disable_registration,
            seed: Seed::from_entropy(),
            // next_user_index: AtomicUsize::new(0),
        };

        env.near_deposit(env.wnear.contract_id(), NearToken::from_near(100))
            .await
            .unwrap();

        env
    }
}

async fn deploy_poa_factory(root: &Near) -> PoaFactoryClient {
    let root_id = root.account_id();
    root.deploy_poa_factory(
        "poa-factory",
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
