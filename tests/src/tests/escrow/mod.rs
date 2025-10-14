use std::sync::LazyLock;

use defuse_escrow::{Contract as EscrowContract, State, TakerMessage};
use defuse_poa_factory::contract::Role as POAFactoryRole;

use near_sdk::NearToken;
use rstest::rstest;
use serde_json::json;

use crate::{
    tests::poa::factory::PoAFactoryExt,
    utils::{Sandbox, account::AccountExt, ft::FtExt, read_wasm},
};

pub static ESCROW_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_escrow"));

#[tokio::test]
#[rstest]
async fn test_escrow() {
    let sandbox = Sandbox::new().await.unwrap();
    let root = sandbox.root_account().clone();

    let maker = sandbox.create_account("maker").await;
    let taker = sandbox.create_account("taker").await;

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

    let maker_token = root
        .poa_factory_deploy_token(poa_factory.id(), "maker-ft", None)
        .await
        .unwrap();
    root.poa_factory_ft_deposit(poa_factory.id(), "maker-ft", maker.id(), 100, None, None)
        .await
        .unwrap();

    let taker_token = root
        .poa_factory_deploy_token(poa_factory.id(), "taker-ft", None)
        .await
        .unwrap();
    root.poa_factory_ft_deposit(poa_factory.id(), "taker-ft", taker.id(), 200, None, None)
        .await
        .unwrap();

    let escrow = root.deploy_contract("escrow", &ESCROW_WASM).await.unwrap();
    escrow
        .call("new")
        .args_json(json!({
            "config": EscrowContract {
                maker_token_id: maker_token.clone(),
                maker_amount: 100,
                taker_token_id: taker_token.clone(),
                taker_amount: 200,
                taker_asset_receiver_id: maker.id().clone(),
                state: State::Init,
                salt: [0; 4],
            }
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // storage_deposits
    for token in [&maker_token, &taker_token] {
        poa_factory
            .ft_storage_deposit_many(token, &[maker.id(), taker.id(), escrow.id()])
            .await
            .unwrap();
    }

    // lock
    maker
        .ft_transfer_call(&maker_token, escrow.id(), 100, None, "")
        .await
        .unwrap();

    // fill
    taker
        .ft_transfer_call(
            &taker_token,
            escrow.id(),
            200,
            None,
            &serde_json::to_string(&TakerMessage {
                receiver_id: taker.id().clone(),
            })
            .unwrap(),
        )
        .await
        .unwrap();
}
