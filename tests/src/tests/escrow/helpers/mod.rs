use anyhow::Result;
use defuse_fees::Pips;
use defuse_sandbox::{
    Sandbox,
    extensions::{
        defuse::{
            DefuseClient,
            contract::{
                contract::config::{DefuseConfig, RolesConfig},
                core::fees::FeesConfig,
            },
        },
        poa::{PoaFactoryClient, PoaFtDepositArgs, contract::contract::Role as PoAFactoryRole},
    },
    near_kit::{AccountId, FungibleToken, Near, NearToken},
    sandbox,
};
use futures::{FutureExt, future::try_join_all, try_join};
use impl_tools::autoimpl;
use near_sdk::{GlobalContractId, json_types::U128};
use rstest::fixture;

use defuse_test_utils::wasms::{DEFUSE_WASM, ESCROW_SWAP_WASM, POA_FACTORY_WASM, WNEAR_WASM};

#[fixture]
pub async fn env(#[future(awt)] sandbox: Sandbox) -> Env {
    Env::new(sandbox).boxed().await
}

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    pub escrow_global_id: GlobalContractId,
    pub verifier: DefuseClient,

    pub src_ft: FungibleToken,
    pub dst_ft: FungibleToken,

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
            async { Ok(Self::deploy_global_escrow_swap(&sandbox).await.unwrap()) },
            async { Ok(Self::deploy_verifier(&sandbox).await.unwrap()) },
            async { Ok(Self::deploy_poa_factory(&sandbox).await.unwrap()) },
            sandbox.generate_sub_account("maker", NearToken::from_near(100)),
            sandbox.generate_sub_account("taker1", NearToken::from_near(100)),
            sandbox.generate_sub_account("taker2", NearToken::from_near(100)),
            sandbox.generate_sub_account("taker3", NearToken::from_near(100)),
            sandbox.generate_sub_account("fee-collector1", NearToken::from_near(0)),
            sandbox.generate_sub_account("fee-collector2", NearToken::from_near(0)),
            sandbox.generate_sub_account("fee-collector3", NearToken::from_near(0)),
        )
        .unwrap();

        let (src_ft, dst_ft) = try_join!(
            sandbox.deploy_ft(&poa_factory, "src-ft"),
            sandbox.deploy_ft(&poa_factory, "dst-ft")
        )
        .unwrap();

        // TODO: revert after storage deposit tx retry is fixed on near kit side
        src_ft
            .storage_deposit(verifier.contract_id())
            .into_future()
            .await
            .unwrap();
        dst_ft
            .storage_deposit(verifier.contract_id())
            .into_future()
            .await
            .unwrap();

        // try_join_all(
        //     [&src_ft, &dst_ft]
        //         .into_iter()
        //         .map(|token| token.storage_deposit(verifier.contract_id()).into_future()),
        // )
        // .await
        // .unwrap();

        try_join_all(
            [
                ("src-ft", maker.account_id().unwrap()),
                ("dst-ft", taker1.account_id().unwrap()),
                ("dst-ft", taker2.account_id().unwrap()),
                ("dst-ft", taker3.account_id().unwrap()),
            ]
            .into_iter()
            .map(|(token, owner_id)| {
                poa_factory
                    .ft_deposit(PoaFtDepositArgs {
                        token: token.to_string(),
                        owner_id: owner_id.clone(),
                        amount: U128(1_000_000),
                        msg: None,
                        memo: None,
                    })
                    .deposit(NearToken::from_millinear(4))
                    .into_future()
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

    async fn deploy_global_escrow_swap(sandbox: &Sandbox) -> anyhow::Result<GlobalContractId> {
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

        Ok(GlobalContractId::AccountId(account_id))
    }

    async fn deploy_verifier(sandbox: &Sandbox) -> anyhow::Result<DefuseClient> {
        let wnear = sandbox
            .deploy_wrap_near("wnear", WNEAR_WASM.clone())
            .await?;

        sandbox
            .deploy_defuse(
                "intents",
                DefuseConfig {
                    wnear_id: wnear.contract_id().clone(),
                    fees: FeesConfig {
                        fee: Pips::ZERO,
                        fee_collector: sandbox.root.account_id().unwrap().clone(),
                    },
                    roles: RolesConfig::default(),
                },
                DEFUSE_WASM.clone(),
            )
            .await
    }

    async fn deploy_poa_factory(sandbox: &Sandbox) -> Result<PoaFactoryClient> {
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
