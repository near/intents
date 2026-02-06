use std::time::Duration;

use defuse_oneshot_condvar::CV_WAIT_GAS;
use defuse_oneshot_condvar::storage::Config as CondVarConfig;
use defuse_sandbox::{Account, FnCallBuilder, OneshotCondVarExt, Sandbox};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn on_auth_call() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();
    let network_config = root.network_config().clone();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    // unauthorized contract (relay vs auth_contract)
    relay
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap_err();

    // unauthorized callee (auth_contract vs relay)
    auth_contract
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": auth_contract.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap_err();

    auth_contract
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn oneshot_condvar_early_notification() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();
    let network_config = root.network_config().clone();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    auth_contract
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    proxy
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("cv_wait")
                .json_args(json!({}))
                .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
        )
        .await
        .unwrap();

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn oneshot_condvar_async_notification_with_promise_resume() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let network_config = root.network_config().clone();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    let authorized = tokio::spawn({
        let condvar_instance = condvar_instance.clone();
        async move {
            proxy
                .tx(condvar_instance.clone())
                .function_call(
                    FnCallBuilder::new("cv_wait").with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
                )
                .await
                .unwrap()
        }
    });

    // replace with waiting for couple blocks
    tokio::time::sleep(Duration::from_secs(3)).await;

    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_ok()
    );
    auth_contract
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    assert!(authorized.await.unwrap().json::<bool>().unwrap());

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn oneshot_condvar_async_notification_timeout() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();
    let network_config = root.network_config().clone();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    let cv_wait = proxy.tx(condvar_instance.clone()).function_call(
        FnCallBuilder::new("cv_wait")
            .json_args(json!({}))
            .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
    );

    let forward_time = sandbox.fast_forward(200);

    let (authorized, ()) = futures::join!(async { cv_wait.await }, forward_time);
    assert!(!authorized.unwrap().json::<bool>().unwrap());

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn oneshot_condvar_retry_after_timeout_with_on_auth() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();
    let network_config = root.network_config().clone();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    // First cv_wait - will timeout
    let cv_wait = proxy.tx(condvar_instance.clone()).function_call(
        FnCallBuilder::new("cv_wait")
            .json_args(json!({}))
            .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
    );

    let forward_time = sandbox.fast_forward(200);

    let (authorized, ()) = futures::join!(async { cv_wait.await }, forward_time);

    // First attempt should timeout and return false
    assert!(!authorized.unwrap().json::<bool>().unwrap());

    // Now call on_auth before second cv_wait
    auth_contract
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("on_auth")
                .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();

    // Second cv_wait should succeed immediately (early authorization path)
    let result = proxy
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("cv_wait")
                .json_args(json!({}))
                .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
        )
        .await
        .unwrap();
    assert!(result.json::<bool>().unwrap());

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn oneshot_condvar_retry_after_timeout_with_on_auth2() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();
    let network_config = root.network_config().clone();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    // First cv_wait - will timeout
    let cv_wait = proxy.tx(condvar_instance.clone()).function_call(
        FnCallBuilder::new("cv_wait")
            .json_args(json!({}))
            .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
    );
    let forward_time = sandbox.fast_forward(200);
    let (authorized, ()) = futures::join!(async { cv_wait.await }, forward_time);
    // First attempt should timeout and return false
    assert!(!authorized.unwrap().json::<bool>().unwrap());

    let cv_wait = proxy.tx(condvar_instance.clone()).function_call(
        FnCallBuilder::new("cv_wait")
            .json_args(json!({}))
            .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
    );

    let (authorized, ()) = futures::join!(async { cv_wait.await }, async {
        tokio::time::sleep(Duration::from_secs(3)).await;
        auth_contract
            .tx(condvar_instance.clone())
            .function_call(
                FnCallBuilder::new("on_auth")
                    .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await
            .unwrap();
    });

    assert!(authorized.unwrap().json::<bool>().unwrap());

    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_err()
    );
}

/// Gas benchmark for cv_wait in worst case (wait first, notify later).
/// This measures gas for the cv_wait transaction that creates a yielded promise.
#[tokio::test]
async fn test_cv_wait_gas_benchmark() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let condvar_global = root.deploy_oneshot_condvar("auth").await;

    let (auth_contract, relay, proxy) = futures::try_join!(
        root.generate_subaccount("auth-contract", INIT_BALANCE),
        root.generate_subaccount("auth-callee", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let network_config = root.network_config().clone();

    let state = CondVarConfig {
        on_auth_caller: auth_contract.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy.id().clone(),
        salt: [0; 32],
    };

    let condvar_instance = root
        .deploy_oneshot_condvar_instance(condvar_global.clone(), state)
        .await;

    // Measure cv_wait gas (worst case: wait first, creates yielded promise)
    let cv_wait_result = proxy
        .tx(condvar_instance.clone())
        .function_call(
            FnCallBuilder::new("cv_wait")
                .json_args(json!({}))
                .with_gas(defuse_oneshot_condvar::CV_WAIT_GAS),
        )
        .exec_transaction();

    // Notify after wait is pending
    let notify_task = async {
        tokio::time::sleep(Duration::from_secs(3)).await;
        auth_contract
            .tx(condvar_instance.clone())
            .function_call(
                FnCallBuilder::new("on_auth")
                    .json_args(json!({ "signer_id": relay.id(), "msg": "" }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await
            .unwrap();
    };

    let (cv_wait_exec_result, ()) = futures::join!(cv_wait_result, notify_task);
    let result = cv_wait_exec_result.unwrap();

    let total_gas = Gas::from_gas(result.total_gas_burnt.as_gas());
    eprintln!("cv_wait total gas consumed: {total_gas:?}");

    // Verify authorization succeeded
    assert!(result.into_result().unwrap().json::<bool>().unwrap());

    // Verify contract was cleaned up
    sandbox.fast_forward(5).await;
    assert!(
        Account::new(condvar_instance.clone(), network_config.clone())
            .view()
            .await
            .is_err()
    );

    assert!(
        CV_WAIT_GAS >= total_gas,
        "CV_WAIT_GAS ({CV_WAIT_GAS:?}) should be >= actual ({total_gas:?})",
    );
}
