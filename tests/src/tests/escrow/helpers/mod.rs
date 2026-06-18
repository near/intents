use defuse_fees::Pips;
use defuse_sandbox::{
    account::Account,
    extensions::{
        defuse::{
            Defuse, DefuseClient, DefuseDeployerExt,
            contract::config::{DefuseConfig, RolesConfig},
            core::fees::FeesConfig,
        },
        poa::{
            PoAFactoryExt, PoaFactoryClient, PoaFactoryDeployerExt,
            contract::Role as PoAFactoryRole,
        },
        wnear::WNearDeployerExt,
    },
    global_contract::GlobalContract,
    kit::{FungibleToken, GlobalContractIdentifier, Near},
    root,
};
use defuse_test_utils::wasms::{DEFUSE_WASM, ESCROW_SWAP_WASM, POA_FACTORY_WASM, WNEAR_WASM};
use futures::{future::try_join_all, try_join};
use impl_tools::autoimpl;
use near_sdk::NearToken;
use rstest::fixture;

#[fixture]
pub async fn env(#[future(awt)] root: Near) -> Env {
    Env::new(root).await
}

#[autoimpl(Deref using self.root)]
pub struct Env {
    pub escrow_global_id: GlobalContractIdentifier,
    pub verifier: DefuseClient,
    pub src_ft: FungibleToken,
    pub dst_ft: FungibleToken,
    pub maker: Near,
    pub takers: [Near; 3],
    pub fee_collectors: [Near; 3],
    root: Near,
}

impl Env {
    pub async fn new(root: Near) -> Self {
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
        ) = futures::join!(
            Self::deploy_global_escrow_swap(&root),
            Self::deploy_verifier(&root),
            Self::deploy_poa_factory(&root),
            root.create_subaccount("maker", NearToken::from_near(100)),
            root.create_subaccount("taker1", NearToken::from_near(100)),
            root.create_subaccount("taker2", NearToken::from_near(100)),
            root.create_subaccount("taker3", NearToken::from_near(100)),
            root.create_subaccount("fee-collector1", None::<NearToken>),
            root.create_subaccount("fee-collector2", None::<NearToken>),
            root.create_subaccount("fee-collector3", None::<NearToken>),
        );

        let (src_ft, dst_ft) = try_join!(
            root.poa_factory_deploy_token(poa_factory.contract_id().clone(), "src-ft", None),
            root.poa_factory_deploy_token(poa_factory.contract_id().clone(), "dst-ft", None),
        )
        .unwrap();

        try_join_all([&src_ft, &dst_ft].into_iter().map(|ft| {
            ft.storage_deposit(verifier.contract_id(), NearToken::from_millinear(2))
                .into_future()
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
                root.poa_factory_ft_deposit(
                    poa_factory.contract_id().clone(),
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
            root,
        }
    }

    async fn deploy_global_escrow_swap(root: &Near) -> GlobalContractIdentifier {
        let account_id = root.account_id().sub_account("escrow-swap").unwrap();
        root.deploy_upgradable_global_contract(
            account_id,
            ESCROW_SWAP_WASM.clone(),
            NearToken::from_near(100),
        )
        .await
        .unwrap()
    }

    async fn deploy_verifier(root: &Near) -> DefuseClient {
        let wnear = root.deploy_wrap_near("wnear", WNEAR_WASM.clone()).await;
        let defuse = root
            .deploy_defuse(
                "intents",
                DefuseConfig {
                    wnear_id: wnear.contract_id().clone(),
                    fees: FeesConfig {
                        fee: Pips::ZERO,
                        fee_collector: root.account_id().clone(),
                    },
                    roles: RolesConfig::default(),
                },
                DEFUSE_WASM.clone(),
            )
            .await;

        defuse.contract::<Defuse>(defuse.account_id())
    }

    async fn deploy_poa_factory(root: &Near) -> PoaFactoryClient {
        root.deploy_poa_factory(
            "poa-factory",
            [root.account_id().clone()],
            [
                (PoAFactoryRole::TokenDeployer, [root.account_id().clone()]),
                (PoAFactoryRole::TokenDepositer, [root.account_id().clone()]),
            ],
            [
                (PoAFactoryRole::TokenDeployer, [root.account_id().clone()]),
                (PoAFactoryRole::TokenDepositer, [root.account_id().clone()]),
            ],
            POA_FACTORY_WASM.clone(),
        )
        .await
    }
}
