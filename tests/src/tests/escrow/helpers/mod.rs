use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
};
use defuse_fees::Pips;
use defuse_poa_factory::contract::Role as PoAFactoryRole;
use defuse_sandbox::{
    Sandbox,
    near_kit::{AccountId, GlobalContractIdentifier, Near, NearToken},
    sandbox,
};
use futures::{FutureExt, future::try_join_all, try_join};
use impl_tools::autoimpl;
use rstest::fixture;

use defuse_test_utils::wasms::{DEFUSE_WASM, ESCROW_SWAP_WASM, POA_FACTORY_WASM, WNEAR_WASM};

#[fixture]
pub async fn env(#[future(awt)] sandbox: Sandbox) -> Env {
    Env::new(sandbox).boxed().await
}

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    pub escrow_global_id: GlobalContractIdentifier,
    pub verifier: AccountId,

    pub src_ft: AccountId,
    pub dst_ft: AccountId,

    pub maker: Near,
    pub takers: [Near; 3],

    pub fee_collectors: [Near; 3],

    sandbox: Sandbox,
}

impl Env {
    pub async fn new(sandbox: Sandbox) -> Self {
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
            Self::deploy_global_escrow_swap(&sandbox),
            Self::deploy_verifier(&sandbox),
            Self::deploy_poa_factory(&sandbox),
            sandbox.generate_subaccount("maker", NearToken::from_near(100)),
            sandbox.generate_subaccount("taker1", NearToken::from_near(100)),
            sandbox.generate_subaccount("taker2", NearToken::from_near(100)),
            sandbox.generate_subaccount("taker3", NearToken::from_near(100)),
            sandbox.generate_subaccount("fee-collector1", NearToken::from_near(0)),
            sandbox.generate_subaccount("fee-collector2", NearToken::from_near(0)),
            sandbox.generate_subaccount("fee-collector3", NearToken::from_near(0)),
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
                ("src-ft", maker.account_id()),
                ("dst-ft", taker1.account_id()),
                ("dst-ft", taker2.account_id()),
                ("dst-ft", taker3.account_id()),
            ]
            .into_iter()
            .map(|(token, owner_id)| {
                sandbox.poa_factory_ft_deposit(
                    poa_factory.account_id().unwrap(),
                    token,
                    owner_id.unwrap(),
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

    async fn deploy_global_escrow_swap(
        sandbox: &Sandbox,
    ) -> anyhow::Result<GlobalContractIdentifier> {
        let account_id: AccountId = format!(
            "escrow-swap.{}",
            sandbox
                .account_id()
                .expect("Sandbox should have an account ID")
                .to_string()
        )
        .parse()?;

        sandbox
            .transaction(&account_id)
            .create_account()
            .transfer(NearToken::from_near(100))
            .publish_contract(ESCROW_SWAP_WASM.clone(), false)
            .await?;

        Ok(GlobalContractIdentifier::AccountId(account_id))
    }

    async fn deploy_verifier(sandbox: &Sandbox) -> anyhow::Result<AccountId> {
        let wnear = sandbox
            .deploy_wrap_near("wnear", WNEAR_WASM.clone())
            .await?;

        sandbox
            .deploy_defuse(
                "intents",
                DefuseConfig {
                    wnear_id: wnear.account_id().unwrap().clone(),
                    fees: FeesConfig {
                        fee: Pips::ZERO,
                        fee_collector: sandbox.account_id().unwrap().clone(),
                    },
                    roles: RolesConfig::default(),
                },
                DEFUSE_WASM.clone(),
            )
            .await?
            .account_id()
            .ok_or(anyhow::anyhow!("Subaccount should be present"))
            .cloned()
    }

    async fn deploy_poa_factory(sandbox: &Sandbox) -> anyhow::Result<Near> {
        let root_id = sandbox
            .account_id()
            .expect("Sandbox should have an account ID")
            .clone();

        sandbox
            .deploy_poa_factory(
                "poa-factory",
                [root_id.clone()],
                [
                    (PoAFactoryRole::TokenDeployer, [root_id.clone()]),
                    (PoAFactoryRole::TokenDepositer, [root_id.clone()]),
                ],
                [
                    (PoAFactoryRole::TokenDeployer, [root_id.clone()]),
                    (PoAFactoryRole::TokenDepositer, [root_id.clone()]),
                ],
                POA_FACTORY_WASM.clone(),
            )
            .await
            .map(Into::into)
    }
}
