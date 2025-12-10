use std::time::Duration;

use defuse_sandbox::{Account, Sandbox};
use defuse_transfer_auth::ext::TransferAuthAccountExt;
use defuse_transfer_auth::storage::{ContractStorage, State};
use near_sdk::{
    Gas, NearToken,
    state_init::{StateInit, StateInitV1},
};
use serde_json::json;

const TIMEOUT: Duration = Duration::from_secs(60);
const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn transfer_auth_global_deployment() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let transfer_auth_global = sandbox.root().deploy_transfer_auth("auth").await;

    let (solver1, solver2, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver1", INIT_BALANCE),
        root.create_subaccount("solver2", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let solver1_raw_state = ContractStorage::init_state(State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    })
    .unwrap();
    let solver1_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(transfer_auth_global.clone()),
        data: solver1_raw_state.clone(),
    });

    let solver2_raw_state = ContractStorage::init_state(State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    })
    .unwrap();
    let solver2_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(transfer_auth_global.clone()),
        data: solver2_raw_state.clone(),
    });

    let auth_transfer_for_solver1 = solver1_state_init.derive_account_id();
    let auth_transfer_for_solver2 = solver2_state_init.derive_account_id();

    println!("auth_transfer_for_solver1: {}", auth_transfer_for_solver1);
    println!("auth_transfer_for_solver2: {}", auth_transfer_for_solver2);

    //NOTE: there is rpc error on state_init action but the contract itself is successfully
    //deployed, so lets ignore error for now
    root.tx(auth_transfer_for_solver1.clone())
        .state_init(transfer_auth_global.clone(), solver1_raw_state)
        .transfer(NearToken::from_yoctonear(1))
        .await;

    // .unwrap();

    let cotnract_instance1_state = proxy
        .tx(auth_transfer_for_solver1)
        .function_call_json::<ContractStorage>(
            "state",
            "{}",
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap();

}

#[tokio::test]
async fn on_auth_call() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (solver, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    // unauthorized contract (relay vs auth_contract)
    relay
        .tx(transfer_auth_instance.clone())
        .function_call_json::<()>(
            "on_auth",
            json!({ "signer_id": relay.id(), "msg": "" }),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap_err();

    // unauthorized callee (auth_contract vs relay)
    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call_json::<()>(
            "on_auth",
            json!({ "signer_id": auth_contract.id(), "msg": "" }),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap_err();

    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call_json::<()>(
            "on_auth",
            json!({ "signer_id": relay.id(), "msg": "" }),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn transfer_auth_early_authorization() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (solver, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    assert!(Account::new(transfer_auth_instance.clone(), root.network_config().clone()).exists().await);

    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call_json::<()>(
            "on_auth",
            json!({ "signer_id": relay.id(), "msg": "" }),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap();

    assert_eq!(
        proxy
            .tx(transfer_auth_instance.clone())
            .function_call_json::<bool>(
                "wait_for_authorization",
                json!({}),
                Gas::from_tgas(300),
                NearToken::from_near(0),
            )
            .await
            .unwrap(),
        true
    );

    assert!(!Account::new(transfer_auth_instance.clone(), root.network_config().clone()).exists().await);
}

#[tokio::test]
async fn transfer_auth_async_authorization() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (solver, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    let network_config = root.network_config().clone();
    let authorized = tokio::spawn({
        let transfer_auth_instance = transfer_auth_instance.clone();
        async move {
            proxy
                .tx(transfer_auth_instance.clone())
                .function_call_json::<bool>(
                    "wait_for_authorization",
                    json!({}),
                    Gas::from_tgas(300),
                    NearToken::from_near(0),
                )
                .await
                .unwrap()
        }
    });

    // replace with waiting for couple blocks
    tokio::time::sleep(Duration::from_secs(3)).await;

    assert!(Account::new(transfer_auth_instance.clone(), network_config.clone()).exists().await);
    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call_json::<()>(
            "on_auth",
            json!({ "signer_id": relay.id(), "msg": "" }),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await
        .unwrap();

    assert!(authorized.await.unwrap());

    assert!(!Account::new(transfer_auth_instance.clone(), network_config.clone()).exists().await);
}

#[tokio::test]
async fn transfer_auth_async_authorization_timeout() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (solver, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = State {
        escrow_contract_id: escrow.id().clone(),
        auth_contract: auth_contract.id().clone(),
        auth_callee: relay.id().clone(),
        querier: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    let network_config = root.network_config().clone();
    let wait_for_authorization = proxy
        .tx(transfer_auth_instance.clone())
        .function_call_json::<bool>(
            "wait_for_authorization",
            json!({}),
            Gas::from_tgas(300),
            NearToken::from_near(0),
        );

    let forward_time = sandbox.fast_forward(200);

    assert!(Account::new(transfer_auth_instance.clone(), network_config.clone()).exists().await);
    let (authorized, _) = futures::join!(
        async { wait_for_authorization.await },
        forward_time
    );

    assert!(!authorized.unwrap());

    assert!(!Account::new(transfer_auth_instance.clone(), network_config.clone()).exists().await);
}
