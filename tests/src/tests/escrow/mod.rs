use std::sync::LazyLock;

use defuse_escrow::{Contract as EscrowContract, State, TakerMessage};
use defuse_poa_factory::contract::Role as POAFactoryRole;

use near_sdk::{AccountId, NearToken, json_types::Base64VecU8};
use near_workspaces::{Account, Contract};
use rstest::rstest;
use serde_json::json;

use crate::{
    tests::poa::factory::PoAFactoryExt,
    utils::{Sandbox, account::AccountExt, ft::FtExt, read_wasm},
};

const FACTORY_GLOBAL_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/factory_contract_global.wasm"
));
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

    const MAKER_AMOUNT: u128 = 100;
    const TAKER_AMOUNT: u128 = 200;

    let maker_token = root
        .poa_factory_deploy_token(poa_factory.id(), "maker-ft", None)
        .await
        .unwrap();
    root.poa_factory_ft_deposit(
        poa_factory.id(),
        "maker-ft",
        maker.id(),
        MAKER_AMOUNT,
        None,
        None,
    )
    .await
    .unwrap();

    let taker_token = root
        .poa_factory_deploy_token(poa_factory.id(), "taker-ft", None)
        .await
        .unwrap();
    root.poa_factory_ft_deposit(
        poa_factory.id(),
        "taker-ft",
        taker.id(),
        TAKER_AMOUNT,
        None,
        None,
    )
    .await
    .unwrap();

    let factory = root
        .deploy_contract("factory", FACTORY_GLOBAL_WASM)
        .await
        .unwrap();

    let escrow_global_contract_id: AccountId =
        format!("escrow-global.{}", factory.id()).parse().unwrap();

    factory
        .call("deploy_global_contract_by_account_id")
        .args_json(json!({
            "name": "escrow",
            "code": Base64VecU8(ESCROW_WASM.clone()),
            "account_id": escrow_global_contract_id,
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let escrow_id: AccountId = format!("escrow-1.{}", factory.id()).parse().unwrap();

    factory
        .call("use_global_contract_by_account")
        .args_json(json!({
            "deployer_account_id": escrow_global_contract_id,
            "account_id": escrow_id,
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let escrow = Contract::from_secret_key(escrow_id, root.secret_key().clone(), sandbox.worker());

    println!("escrow: {}", escrow.id());

    // let escrow = root.deploy_contract("escrow", &ESCROW_WASM).await.unwrap();
    // escrow
    //     .call("new")
    //     .args_json(json!({
    //         "params": Params {
    //             maker_id: maker.id().clone(),
    //             src_asset: maker_token.clone(),
    //             maker_amount: MAKER_AMOUNT,
    //             taker_token_id: taker_token.clone(),
    //             taker_amount: TAKER_AMOUNT,
    //             receiver_id: None,
    //             receiver_memo: None,
    //             receiver_msg: None,
    //             state: State::Init,
    //             salt: [0; 4],
    //         }
    //     }))
    //     .max_gas()
    //     .transact()
    //     .await
    //     .unwrap()
    //     .into_result()
    //     .unwrap();

    // // storage_deposits
    // for token in [&maker_token, &taker_token] {
    //     poa_factory
    //         .ft_storage_deposit_many(token, &[maker.id(), taker.id(), escrow.id()])
    //         .await
    //         .unwrap();
    // }

    // // lock
    // maker
    //     .ft_transfer_call(&maker_token, escrow.id(), MAKER_AMOUNT, None, "")
    //     .await
    //     .unwrap();

    // // fill
    // taker
    //     .ft_transfer_call(
    //         &taker_token,
    //         escrow.id(),
    //         TAKER_AMOUNT,
    //         None,
    //         &serde_json::to_string(&TakerMessage {
    //             receiver_id: taker.id().clone(),
    //             memo: None,
    //             msg: None,
    //         })
    //         .unwrap(),
    //     )
    //     .await
    //     .unwrap();

    // assert_eq!(
    //     sandbox
    //         .ft_token_balance_of(&maker_token, taker.id())
    //         .await
    //         .unwrap(),
    //     MAKER_AMOUNT,
    // );
    // assert_eq!(
    //     sandbox
    //         .ft_token_balance_of(&taker_token, maker.id())
    //         .await
    //         .unwrap(),
    //     TAKER_AMOUNT,
    // );
    // escrow
    //     .view_account()
    //     .await
    //     .expect_err("escrow deletes itself after being filled");
}
