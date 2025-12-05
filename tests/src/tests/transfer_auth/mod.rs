//! Integration tests for the transfer-auth contract.
//!
//! This module tests the authorization mechanism with yielded promises,
//! covering scenarios where on_auth arrives before/after wait_for_authorization.

use crate::{
    tests::defuse::{
        DefuseSignerExt,
        accounts::AccountManagerExt,
        env::{Env, get_account_public_key},
    },
    utils::{account::AccountExt, read_wasm, test_log::TestLog},
};
use defuse::core::intents::auth::AuthCall;
use defuse_transfer_auth::AuthMessage;
use near_sdk::{NearToken, env::sha256};
use serde_json::json;
use std::sync::LazyLock;

static TRANSFER_AUTH_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/transfer-auth/defuse_transfer_auth"));
const ESCROW_ID: &str = "dummy_instance.escrow.near";

/// Helper function to deploy and initialize the transfer-auth contract
async fn setup_transfer_auth(
    env: &Env,
    name: &str,
    solver: &near_workspaces::Account,
    relay: &near_workspaces::Account,
    querier: &near_workspaces::Account,
    //TODO: update init params
    _escrow_params_hash: [u8; 32],
) -> near_workspaces::Contract {
    // Deploy the contract
    let contract = env
        .deploy_contract(name, &TRANSFER_AUTH_WASM)
        .await
        .unwrap();

    // Initialize with TransferCallStateInit
    contract
        .call("new")
        .args_json(json!({
            "state_init": {
                "solver_id": solver.id(),
                "escrow_contract_id": ESCROW_ID.parse::<near_sdk::AccountId>().unwrap(),
                "auth_contract": env.defuse.id(),
                "auth_callee": relay.id(),
                "querier": querier.id(),
            }
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // Register contract's public key in defuse (needed to receive AuthCall)
    contract
        .as_account()
        .add_public_key(
            env.defuse.id(),
            get_account_public_key(contract.as_account()),
        )
        .await
        .unwrap();

    contract
}

/// Helper function to create a JSON AuthMessage
fn create_auth_message(solver_id: &near_sdk::AccountId, escrow_params_hash: [u8; 32]) -> String {
    let auth_msg = AuthMessage {
        solver_id: solver_id.clone(),
        escrow_params_hash,
    };
    serde_json::to_string(&auth_msg).unwrap()
}

/// Helper function to trigger on_auth via AuthCall intent through defuse
async fn trigger_on_auth(
    env: &Env,
    relay: &near_workspaces::Account,
    transfer_auth_contract: &near_workspaces::Contract,
    auth_message: String,
) -> anyhow::Result<()> {
    // Create AuthCall intent
    let auth_call_intent = AuthCall {
        contract_id: transfer_auth_contract.id().clone(),
        msg: auth_message,
        attached_deposit: NearToken::from_yoctonear(0),
        min_gas: None,
    };

    // Sign as relay (who is the authorizer_entity)
    let auth_call_payload = relay
        .sign_defuse_payload_default(env.defuse.id(), [auth_call_intent])
        .await?;

    // Execute through defuse contract
    relay
        .call(env.defuse.id(), "execute_intents")
        .args_json(json!({
            "signed": vec![auth_call_payload],
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

/// Test 1: on_auth called BEFORE wait_for_authorization
/// Expected: wait_for_authorization returns true immediately
#[tokio::test]
async fn transfer_auth_early_authorization() {
    // Setup environment
    let env = Env::builder().create_unique_users().build().await;
    let (solver, relay, querier) =
        futures::join!(env.create_user(), env.create_user(), env.create_user());

    // Create unique escrow params hash
    let escrow_params_hash: [u8; 32] = sha256(b"test_escrow_params_early").try_into().unwrap();

    // Deploy and initialize transfer-auth contract
    let transfer_auth = setup_transfer_auth(
        &env,
        "transfer-auth-early",
        &solver,
        &relay,
        &querier,
        escrow_params_hash,
    )
    .await;

    // Trigger on_auth FIRST (sets authorized = true)
    let auth_message = create_auth_message(solver.id(), escrow_params_hash);
    trigger_on_auth(&env, &relay, &transfer_auth, auth_message)
        .await
        .unwrap();

    // Call wait_for_authorization - should return true immediately
    let result = querier
        .call(transfer_auth.id(), "wait_for_authorization")
        .max_gas()
        .transact()
        .await
        .unwrap();

    // Verify result is true (not a promise, immediate return)
    let authorized: bool = result.json().unwrap();
    assert!(authorized, "Should be authorized immediately");
}

/// Test 2: wait_for_authorization called FIRST, then on_auth
/// Expected: Promise gets created, then resumed by on_auth, authorized becomes true
#[tokio::test]
async fn transfer_auth_async_authorization() {
    println!("test start ");
    // Setup environment
    let env = Env::builder().create_unique_users().build().await;
    let (solver, relay, querier) =
        futures::join!(env.create_user(), env.create_user(), env.create_user());

    // Create unique escrow params hash
    let escrow_params_hash: [u8; 32] = sha256(b"test_escrow_params_async").try_into().unwrap();

    // Deploy and initialize transfer-auth contract
    let transfer_auth = setup_transfer_auth(
        &env,
        "transfer-auth-async",
        &solver,
        &relay,
        &querier,
        escrow_params_hash,
    )
    .await;
    let transfer_auth_id_clone = transfer_auth.id().clone();

    println!("wait_for_authorization started");
    let wait_result = tokio::spawn(async move {
        // Call wait_for_authorization FIRST (creates yielded promise)
        querier
            .call(&transfer_auth_id_clone, "wait_for_authorization")
            .max_gas()
            .transact()
            .await
            .unwrap()
    });

    println!("wait_for_authorization finished");

    // Trigger on_auth AFTER wait_for_authorization (resumes the promise)
    let auth_message = create_auth_message(solver.id(), escrow_params_hash);
    trigger_on_auth(&env, &relay, &transfer_auth, auth_message)
        .await
        .unwrap();

    let execution_result = wait_result.await.unwrap().into_result().unwrap();

    execution_result.logs().iter().for_each(|log| {
        println!("{}", log);
    });
    // let authorized: bool = wait_result.json().unwrap();
    assert!(
        execution_result.json::<bool>().unwrap(),
        "Should be authorized after on_auth"
    );
    //
    // assert!(authorized, "Should be authorized after on_auth");
}

/// Test 3: wait_for_authorization called but NO on_auth received
/// Expected: After 200+ blocks, promise auto-resumes, authorized remains false
#[tokio::test]
async fn transfer_auth_timeout() {
    // Setup environment
    let env = Env::builder().create_unique_users().build().await;
    let (solver, relay, querier) =
        futures::join!(env.create_user(), env.create_user(), env.create_user());

    // Create unique escrow params hash
    let escrow_params_hash: [u8; 32] = sha256(b"test_escrow_params_timeout").try_into().unwrap();

    // Deploy and initialize transfer-auth contract
    let transfer_auth = setup_transfer_auth(
        &env,
        "transfer-auth-timeout",
        &solver,
        &relay,
        &querier,
        escrow_params_hash,
    )
    .await;

    println!("start wait_for_authorization");
    // Call wait_for_authorization (creates yielded promise with 200 block timeout)
    let wait_result = tokio::spawn(async move {
        querier
            .call(transfer_auth.id(), "wait_for_authorization")
            .max_gas()
            .transact_async()
            .await
            .unwrap()
    });
    println!("end wait_for_authorization");

    println!("current block : {}", env.sandbox().block_height().await);
    // Fast forward MORE than 200 blocks to trigger timeout
    env.sandbox().skip_blocks(500).await;
    println!(
        "blocks skipped current block : {}",
        env.sandbox().block_height().await
    );

    let task_result = wait_result.await.unwrap();
    let execution_result = task_result.into_future().await.unwrap();

    execution_result.logs().iter().for_each(|log| {
        println!("{}", log);
    });

    // let execution_result = task_result.into_result().unwrap();
    let bytes = execution_result.raw_bytes().unwrap();
    let string_result = String::from_utf8(bytes).unwrap();
    println!("{string_result}");

    // let task_result = wait_result.await.unwrap();
    // assert!(!task_result.into_result().unwrap().json::<bool>().unwrap());
}

#[tokio::test]
async fn transfer_auth_timeout_blocking() {
    // Setup environment
    let env = Env::builder().create_unique_users().build().await;
    let (solver, relay, querier) =
        futures::join!(env.create_user(), env.create_user(), env.create_user());

    // Create unique escrow params hash
    let escrow_params_hash: [u8; 32] = sha256(b"test_escrow_params_timeout").try_into().unwrap();

    // Deploy and initialize transfer-auth contract
    let transfer_auth = setup_transfer_auth(
        &env,
        "transfer-auth-timeout",
        &solver,
        &relay,
        &querier,
        escrow_params_hash,
    )
    .await;

    println!("start wait_for_authorization");
    // Call wait_for_authorization (creates yielded promise with 200 block timeout)
    println!(
        "blocks current block : {}",
        env.sandbox().block_height().await
    );
    let wait_result = querier
        .call(transfer_auth.id(), "wait_for_authorization")
        .max_gas()
        .transact_async()
        .await
        .unwrap();
    println!("end wait_for_authorization");

    // Fast forward MORE than 200 blocks to trigger timeout
    // env.sandbox().skip_blocks(205).await;
    //
    //
    let task_result = wait_result.into_future().await.unwrap();

    println!(
        "blocks skipped current block : {}",
        env.sandbox().block_height().await
    );

    let execution_result = task_result.into_result().unwrap();

    execution_result.logs().iter().for_each(|log| {
        println!("{}", log);
    });

    let bytes = execution_result.raw_bytes().unwrap();
    let string_result = String::from_utf8(bytes).unwrap();
    println!("{string_result}");

    // let task_result = wait_result.await.unwrap();
    // let execution_result = task_result.into_result().unwrap();
    // println!("{execution_result:?}");
    // assert!(!execution_result.json::<bool>().unwrap());
}
