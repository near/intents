use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::tests::defuse::env::Env;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage};
use defuse_sandbox::extensions::storage_management::StorageManagementExt;
use defuse_sandbox::{Account, FnCallBuilder, FtExt, FtViewExt, MtExt, MtViewExt};
use defuse_sandbox_ext::{EscrowProxyExt, MtReceiverStubAccountExt, OneshotCondVarAccountExt};
use defuse_oneshot_condvar::CondVarContext;
use defuse_oneshot_condvar::storage::{ContractStorage, StateInit as CondVarStateInit};
use multi_token_receiver_stub::{FTReceiverMode, MTReceiverMode};
use near_sdk::AccountId;
use near_sdk::{
    Gas, GlobalContractId, NearToken,
    json_types::U128,
    state_init::{StateInit, StateInitV1},
};

/// Derive the oneshot-condvar instance account ID from its state
pub fn derive_oneshot_condvar_account_id(
    global_contract_id: &GlobalContractId,
    state: &CondVarStateInit,
) -> AccountId {
    let raw_state = ContractStorage::init_state(state.clone()).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: global_contract_id.clone(),
        data: raw_state,
    });
    state_init.derive_account_id()
}


#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_proxy_returns_funds_on_timeout_of_authorization() {
    let env = Env::builder().build().await;
    let (condvar_global, mt_receiver_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root()
            .deploy_mt_receiver_stub_global("mt_receiver_global"),
    );
    let mt_receiver_instance = env
        .root()
        .deploy_mt_receiver_stub_instance(mt_receiver_global.clone(), BTreeMap::default())
        .await;

    let (solver, relay, proxy) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    // Setup proxy
    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };
    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(condvar_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(mt_receiver_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };

    proxy.deploy_escrow_proxy(roles, config).await.unwrap();

    // Create MT token with initial balance for solver
    let initial_amount = 1_000_000u128;
    let (_, token_id) = env
        .create_mt_token_with_initial_balances([(solver.id().clone(), initial_amount)])
        .await
        .unwrap();


    let transfer_msg = TransferMessage {
        receiver_id: mt_receiver_instance.clone(),
        salt: [1u8; 32],
        msg: String::new(),
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();
    let token_id_str = token_id.to_string();
    let (transfer_result, ()) = futures::join!(
        solver.mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &token_id_str,
            initial_amount / 2,
            None,
            &msg_json,
        ),
        env.sandbox().fast_forward(250)
    );

    assert_eq!(
        transfer_result.unwrap(),
        0,
        "Used amount should be 0 when transfer times out and refunds"
    );

    assert_eq!(
        initial_amount,
        env.defuse
            .mt_balance_of(solver.id(), &token_id_str)
            .await
            .unwrap(),
        "Solver balance should be unchanged after timeout refund"
    );
}

/// Test that transfer succeeds when relay authorizes via on_auth call
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_transfer_authorized_by_relay() {
    let env = Env::builder().build().await;

    // Deploy global contracts in parallel
    let (condvar_global, mt_receiver_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root().deploy_mt_receiver_stub_global("mt_receiver_global"),
    );

    // Create accounts in parallel
    let (solver, relay, proxy) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    // Use root as auth_contract since we need signing capability for on_auth call
    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(condvar_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(mt_receiver_global.clone()),
        auth_contract: env.root().id().clone(),
        auth_collee: relay.id().clone(),
    };

    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();

    // Derive and pre-deploy the escrow instance (mt-receiver-stub)
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(mt_receiver_global.clone()),
        data: BTreeMap::new(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    // Deploy escrow instance via state_init
    env.root()
        .tx(escrow_instance_id.clone())
        .state_init(mt_receiver_global.clone(), BTreeMap::new())
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    // Create token with initial balance for solver
    let initial_balance: u128 = 1_000_000;
    let (_, token_id) = env
        .create_mt_token_with_initial_balances([(solver.id().clone(), initial_balance)])
        .await
        .unwrap();

    // Record initial solver balance
    let initial_solver_balance = env
        .defuse
        .mt_balance_of(solver.id(), &token_id.to_string())
        .await
        .unwrap();

    let inner_msg_json = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();

    // Build TransferMessage for the proxy (wraps inner message)
    let transfer_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [2u8; 32], // Different salt from timeout test
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let proxy_transfer_amount: u128 = 100_000;

    // Derive the transfer-auth instance address (same logic as proxy uses)
    // The hash is computed from CondVarContext, not the message itself
    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id()),
        token_ids: Cow::Owned(vec![token_id.to_string()]),
        amounts: Cow::Owned(vec![U128(proxy_transfer_amount)]),
        salt: transfer_msg.salt,
        msg: Cow::Borrowed(&msg_json),
    }
    .hash();

    let auth_state = CondVarStateInit {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: config.auth_contract.clone(),
        on_auth_signer: config.auth_collee.clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    let token_id_str = token_id.to_string();

    // Build raw state for state_init (same as proxy does)
    let raw_state = ContractStorage::init_state(auth_state).unwrap();

    // Run transfer and on_auth call concurrently
    // Transfer starts the yield promise, on_auth authorizes it
    let (_transfer_result, _auth_result) = futures::join!(
        solver.mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &token_id_str,
            proxy_transfer_amount,
            None,
            &msg_json,
        ),
        // Call on_auth from root (auth_contract) with relay as signer_id
        // Include state_init to deploy the transfer-auth instance if not already deployed
        async {
            env.root()
                .tx(condvar_instance_id.clone())
                .state_init(condvar_global.clone(), raw_state)
                .function_call(
                    FnCallBuilder::new("on_auth")
                        .json_args(serde_json::json!({
                            "signer_id": relay.id(),
                            "msg": "",
                        }))
                        .with_gas(Gas::from_tgas(50))
                        .with_deposit(NearToken::from_yoctonear(1)),
                )
                .await
                .unwrap();
        }
    );

    // Verify solver balance decreased
    let final_solver_balance = env
        .defuse
        .mt_balance_of(solver.id(), &token_id_str)
        .await
        .unwrap();

    assert_eq!(
        initial_solver_balance - proxy_transfer_amount,
        final_solver_balance,
        "Solver balance should decrease by transferred amount"
    );

    // Verify escrow instance received the tokens
    let escrow_balance = env
        .defuse
        .mt_balance_of(&escrow_instance_id, &token_id_str)
        .await
        .unwrap();

    assert_eq!(
        escrow_balance, proxy_transfer_amount,
        "Escrow instance should have received the transferred tokens"
    );
}

/// Test that proxy's authorize function can authorize a transfer-auth instance
#[tokio::test]
async fn test_proxy_authorize_function() {
    let env = Env::builder().build().await;

    // Deploy global contracts in parallel
    let (condvar_global, mt_receiver_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root().deploy_mt_receiver_stub_global("mt_receiver_global"),
    );

    let (relay, proxy) = futures::join!(
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::from([(
            defuse_escrow_proxy::Role::Canceller,
            HashSet::from([relay.id().clone()]),
        )]),
    };

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(condvar_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(mt_receiver_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };

    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();

    let test_msg_hash = [42u8; 32];
    let auth_state = CondVarStateInit {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: config.auth_contract.clone(),
        on_auth_signer: proxy.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: test_msg_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    let raw_state = ContractStorage::init_state(auth_state).unwrap();
    env.root()
        .tx(condvar_instance_id.clone())
        .state_init(condvar_global.clone(), raw_state)
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    let authorize_result = relay
        .tx(proxy.id().clone())
        .function_call(
            FnCallBuilder::new("authorize")
                .json_args(serde_json::json!({
                    "condvar_id": condvar_instance_id,
                }))
                .with_gas(Gas::from_tgas(100))
                .with_deposit(NearToken::from_yoctonear(1)),
        )
        .await;

    assert!(
        authorize_result.is_ok(),
        "authorize should succeed when called by account with Canceller role"
    );

    let condvar_account =
        Account::new(condvar_instance_id, env.root().network_config().clone());
    let cv_is_notified: bool = condvar_account
        .call_view_function_json("cv_is_notified", serde_json::json!({}))
        .await
        .unwrap();

    assert!(
        cv_is_notified,
        "OneshotCondVar instance should be authorized after proxy.authorize() call"
    );
}

/// Test that FT transfer succeeds when relay authorizes via on_auth call
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_ft_transfer_authorized_by_relay() {
    let env = Env::builder().build().await;

    // Deploy global contracts in parallel
    let (condvar_global, ft_receiver_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root()
            .deploy_mt_receiver_stub_global("ft_receiver_global"),
    );

    // Create accounts in parallel
    let (solver, relay, proxy) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    // Use root as auth_contract since we need signing capability for on_auth call
    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(condvar_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(ft_receiver_global.clone()),
        auth_contract: env.root().id().clone(),
        auth_collee: relay.id().clone(),
    };

    proxy
        .deploy_escrow_proxy(roles, config.clone())
        .await
        .unwrap();

    // Derive and pre-deploy the escrow instance (ft-receiver-stub)
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(ft_receiver_global.clone()),
        data: BTreeMap::new(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    // Deploy escrow instance via state_init
    env.root()
        .tx(escrow_instance_id.clone())
        .state_init(ft_receiver_global.clone(), BTreeMap::new())
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    // Create FT token with initial balance for solver
    let initial_balance: u128 = 1_000_000;
    let (ft_token, _) = env
        .create_ft_token_with_initial_balances([(solver.id().clone(), initial_balance)])
        .await
        .unwrap();

    // Storage deposit for proxy and escrow on the FT token
    let (proxy_storage, escrow_storage) = futures::join!(
        solver.storage_deposit(ft_token.id(), Some(proxy.id().as_ref()), NearToken::from_near(1)),
        solver.storage_deposit(
            ft_token.id(),
            Some(escrow_instance_id.as_ref()),
            NearToken::from_near(1)
        ),
    );
    proxy_storage.unwrap();
    escrow_storage.unwrap();

    // Record initial solver balance
    let initial_solver_balance = ft_token.ft_balance_of(solver.id()).await.unwrap();

    let inner_msg_json = serde_json::to_string(&FTReceiverMode::AcceptAll).unwrap();

    // Build TransferMessage for the proxy (wraps inner message)
    let transfer_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [3u8; 32], // Different salt from other tests
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let proxy_transfer_amount: u128 = 100_000;

    // Derive the transfer-auth instance address (same logic as proxy uses)
    // For FT, token_ids is a single-element vec with the FT contract account as string
    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id()),
        token_ids: Cow::Owned(vec![ft_token.id().to_string()]),
        amounts: Cow::Owned(vec![U128(proxy_transfer_amount)]),
        salt: transfer_msg.salt,
        msg: Cow::Borrowed(&msg_json),
    }
    .hash();

    let auth_state = CondVarStateInit {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: config.auth_contract.clone(),
        on_auth_signer: config.auth_collee.clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    // Build raw state for state_init (same as proxy does)
    let raw_state = ContractStorage::init_state(auth_state).unwrap();

    // Run transfer and on_auth call concurrently
    // Transfer starts the yield promise, on_auth authorizes it
    let (_transfer_result, _auth_result) = futures::join!(
        solver.ft_transfer_call(
            ft_token.id(),
            proxy.id(),
            proxy_transfer_amount,
            None,
            &msg_json,
        ),
        // Call on_auth from root (auth_contract) with relay as signer_id
        // Include state_init to deploy the transfer-auth instance if not already deployed
        async {
            env.root()
                .tx(condvar_instance_id.clone())
                .state_init(condvar_global.clone(), raw_state)
                .function_call(
                    FnCallBuilder::new("on_auth")
                        .json_args(serde_json::json!({
                            "signer_id": relay.id(),
                            "msg": "",
                        }))
                        .with_gas(Gas::from_tgas(50))
                        .with_deposit(NearToken::from_yoctonear(1)),
                )
                .await
                .unwrap();
        }
    );

    // Verify solver balance decreased
    let final_solver_balance = ft_token.ft_balance_of(solver.id()).await.unwrap();

    assert_eq!(
        initial_solver_balance - proxy_transfer_amount,
        final_solver_balance,
        "Solver balance should decrease by transferred amount"
    );

    // Verify escrow instance received the tokens
    let escrow_balance = ft_token.ft_balance_of(&escrow_instance_id).await.unwrap();

    assert_eq!(
        escrow_balance, proxy_transfer_amount,
        "Escrow instance should have received the transferred tokens"
    );
}
