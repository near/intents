use std::time::Duration;

use defuse_sandbox::{Account, FnCallBuilder, Sandbox, UnwrapGlobalContractDeployment};
use defuse_sandbox_ext::TransferAuthAccountExt;
use defuse_transfer_auth::storage::{ContractStorage, StateInit as TransferAuthStateInit};
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};
const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn transfer_auth_global_deployment() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = sandbox.root().deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let solver1_raw_state = ContractStorage::init_state(TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    })
    .unwrap();
    let solver1_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(transfer_auth_global.clone()),
        data: solver1_raw_state.clone(),
    });

    let solver2_raw_state = ContractStorage::init_state(TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    })
    .unwrap();
    let solver2_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(transfer_auth_global.clone()),
        data: solver2_raw_state.clone(),
    });

    let auth_transfer_for_solver1 = solver1_state_init.derive_account_id();
    let auth_transfer_for_solver2 = solver2_state_init.derive_account_id();

    println!("auth_transfer_for_solver1: {auth_transfer_for_solver1}");
    println!("auth_transfer_for_solver2: {auth_transfer_for_solver2}");

    let result = root
        .tx(auth_transfer_for_solver1.clone())
        .state_init(transfer_auth_global.clone(), solver1_raw_state)
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap_global_contract_deployment();
    assert!(result, "state_init should succeed");

    let account = Account::new(auth_transfer_for_solver1, proxy.network_config().clone());
    let _: ContractStorage = account.call_view_function_json("view", json!({})).await.unwrap();
}

#[tokio::test]
async fn on_auth_call() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    // unauthorized contract (relay vs auth_contract)
    relay
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap_err();

    // unauthorized callee (auth_contract vs relay)
    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": auth_contract.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap_err();

    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn transfer_auth_early_authorization() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    // assert!(Account::new(transfer_auth_instance.clone(), root.network_config().clone()).view().await.is_ok());

    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    // assert!(!Account::new(transfer_auth_instance.clone(), root.network_config().clone()).view().await.is_ok());
}

#[tokio::test]
async fn transfer_auth_async_authorization() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let network_config = root.network_config().clone();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    let authorized = tokio::spawn({
        let transfer_auth_instance = transfer_auth_instance.clone();
        async move {
            proxy
                .tx(transfer_auth_instance.clone())
                .function_call(
                    FnCallBuilder::new("wait_for_authorization")
                        .json_args(json!({}))
                        .with_gas(Gas::from_tgas(300)),
                )
                .await
                .unwrap()
        }
    });

    // replace with waiting for couple blocks
    tokio::time::sleep(Duration::from_secs(3)).await;

    assert!(
        Account::new(transfer_auth_instance.clone(), network_config.clone())
            .view()
            .await
            .is_ok()
    );
    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    authorized.await.unwrap();
}

#[tokio::test]
async fn transfer_auth_async_authorization_timeout() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    let wait_for_authorization = proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        );

    let forward_time = sandbox.fast_forward(200);

    // assert!(Account::new(transfer_auth_instance.clone(), network_config.clone()).view().await.is_ok());
    let (authorized, ()) = futures::join!(async { wait_for_authorization.await }, forward_time);

    // Timeout returns false (not authorized)
    assert!(authorized.unwrap().json::<bool>().is_ok());

    // Contract should still exist after timeout (state reset to Idle for retry)
    // assert!(Account::new(transfer_auth_instance.clone(), network_config.clone()).view().await.is_ok());
}

#[tokio::test]
async fn transfer_auth_retry_after_timeout_with_on_auth() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    // First wait_for_authorization - will timeout
    let wait_for_authorization = proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        );

    let forward_time = sandbox.fast_forward(200);

    let (authorized, ()) = futures::join!(async { wait_for_authorization.await }, forward_time);

    // First attempt should timeout and return false
    assert!(authorized.unwrap().json::<bool>().is_ok());

    // Now call on_auth before second wait_for_authorization
    auth_contract
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    // Second wait_for_authorization should succeed immediately (early authorization path)
    let _ = proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn transfer_auth_retry_after_timeout_with_on_auth2() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let transfer_auth_global = root.deploy_transfer_auth("auth").await;

    let (escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(escrow.id().clone()),
        auth_contract: auth_contract.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: [0; 32],
    };

    let transfer_auth_instance = root
        .deploy_transfer_auth_instance(transfer_auth_global.clone(), state)
        .await;

    // First wait_for_authorization - will timeout
    let wait_for_authorization = proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        );
    let forward_time = sandbox.fast_forward(200);
    let (authorized, ()) = futures::join!(async { wait_for_authorization.await }, forward_time);
    // First attempt should timeout and return false
    assert!(authorized.unwrap().json::<bool>().is_ok());

    let wait_for_authorization = proxy
        .tx(transfer_auth_instance.clone())
        .function_call(
            FnCallBuilder::new("wait_for_authorization")
                .json_args(json!({}))
                .with_gas(Gas::from_tgas(300)),
        );

    let (authorized, ()) = futures::join!(async { wait_for_authorization.await }, async {
        tokio::time::sleep(Duration::from_secs(3)).await;
        auth_contract
            .tx(transfer_auth_instance.clone())
            .function_call(
                FnCallBuilder::new("on_auth")
                    .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await
            .unwrap();
    });

    authorized.unwrap();
}
