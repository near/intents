use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
    sandbox_ext::deployer::DefuseExt,
};
use defuse_escrow_swap::Pips;
use defuse_poa_factory::{
    contract::Role as PoAFactoryRole,
    sandbox_ext::{PoAFactoryDeployerExt, PoAFactoryExt},
};
use defuse_sandbox::{
    Account, Sandbox, SigningAccount, anyhow,
    api::types::transaction::actions::GlobalContractDeployMode,
    extensions::{storage_management::StorageManagementExt, wnear::WNearDeployerExt},
    sandbox,
};
use futures::{future::try_join_all, try_join};
use impl_tools::autoimpl;
use near_sdk::{GlobalContractId, NearToken};
use rstest::fixture;

use defuse_escrow_swap::sandbox_ext::ESCROW_SWAP_WASM;

#[fixture]
pub async fn env() -> Env {
    Env::new().await
}

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    pub escrow_global_id: GlobalContractId,
    pub verifier: Account,

    pub src_ft: Account,
    pub dst_ft: Account,

    pub maker: SigningAccount,
    pub takers: [SigningAccount; 3],

    pub fee_collectors: [SigningAccount; 3],

    sandbox: Sandbox,
}

impl Env {
    pub async fn new() -> Self {
        let sandbox = sandbox(NearToken::from_near(100_000)).await;
        let root = sandbox.root();

        let (
            escrow_global_id,
            verifier,
            poa_factory,
            maker,
            taker1,
            taker2,
            taker3,
            fee_collector1,
            fee_collector2,
            fee_collector3,
        ) = try_join!(
            Self::deploy_global_escrow_swap(root),
            Self::deploy_verifier(root),
            Self::deploy_poa_factory(root),
            root.generate_subaccount("maker", NearToken::from_near(100)),
            root.generate_subaccount("taker1", NearToken::from_near(100)),
            root.generate_subaccount("taker2", NearToken::from_near(100)),
            root.generate_subaccount("taker3", NearToken::from_near(100)),
            root.generate_subaccount("fee-collector1", None),
            root.generate_subaccount("fee-collector2", None),
            root.generate_subaccount("fee-collector3", None),
        )
        .unwrap();

        let (src_ft, dst_ft) = try_join!(
            root.poa_factory_deploy_token(poa_factory.id(), "src-ft", None),
            root.poa_factory_deploy_token(poa_factory.id(), "dst-ft", None),
        )
        .unwrap();

        try_join_all([src_ft.id(), dst_ft.id()].into_iter().map(|token| {
            root.storage_deposit(token, verifier.id().as_ref(), NearToken::from_millinear(2))
        }))
        .await
        .unwrap();

        try_join_all(
            [
                ("src-ft", maker.id()),
                ("dst-ft", taker1.id()),
                ("dst-ft", taker2.id()),
                ("dst-ft", taker3.id()),
            ]
            .into_iter()
            .map(|(token, owner_id)| {
                root.poa_factory_ft_deposit(
                    poa_factory.id(),
                    token,
                    owner_id,
                    1_000_000,
                    None,
                    None,
                )
            }),
        )
        .await
        .unwrap();

        Self {
            escrow_global_id,
            verifier,
            src_ft,
            dst_ft,
            maker,
            takers: [taker1, taker2, taker3],
            fee_collectors: [fee_collector1, fee_collector2, fee_collector3],
            sandbox,
        }
    }

    async fn deploy_global_escrow_swap(root: &SigningAccount) -> anyhow::Result<GlobalContractId> {
        let account_id = root.id().sub_account("escrow-swap").unwrap();
        root.tx(account_id.clone())
            .create_account()
            .transfer(NearToken::from_near(100))
            .deploy_global(
                ESCROW_SWAP_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await?;
        Ok(GlobalContractId::AccountId(account_id))
    }

    async fn deploy_verifier(root: &SigningAccount) -> anyhow::Result<Account> {
        let wnear = root.deploy_wrap_near("wnear").await?;

        root.deploy_defuse(
            "intents",
            DefuseConfig {
                wnear_id: wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: root.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .map(Into::into)
    }

    async fn deploy_poa_factory(root: &SigningAccount) -> anyhow::Result<Account> {
        root.deploy_poa_factory(
            "poa-factory",
            [root.id().clone()],
            [
                (PoAFactoryRole::TokenDeployer, [root.id().clone()]),
                (PoAFactoryRole::TokenDepositer, [root.id().clone()]),
            ],
            [
                (PoAFactoryRole::TokenDeployer, [root.id().clone()]),
                (PoAFactoryRole::TokenDepositer, [root.id().clone()]),
            ],
        )
        .await
        .map(Into::into)
    }
}
