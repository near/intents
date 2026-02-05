//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::time::Duration;

use crate::env::Env;
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use crate::utils::escrow_builders::ParamsBuilder;
use crate::utils::escrow_builders::{FillMessageBuilder, FundMessageBuilder};
use defuse_core::Deadline;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_core::token_id::TokenId;
use defuse_core::token_id::nep245::Nep245TokenId;
use defuse_escrow_proxy::CondVarContext;
use defuse_escrow_proxy::{ProxyConfig, TransferMessage as ProxyTransferMessage};
use defuse_oneshot_condvar::storage::{Config as CondVarConfig, ContractStorage as CondVarStorage};
use defuse_sandbox::{EscrowProxyExt, EscrowSwapExt, MtExt, MtViewExt, OneshotCondVarExt};
use near_sdk::serde_json;

use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{GlobalContractId, NearToken};

/// Test full escrow swap flow with proxy authorization
#[tokio::test]
async fn test_escrow_swap_with_proxy_full_flow() {
    let swap_amount: u128 = 100_000_000; // Fits within ft_deposit_to_root mint limit (1e9)
    let env = Env::builder().build().await;
    let (condvar_global, escrow_swap_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("oneshot_condvar"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );
    let (maker, solver, relay) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), swap_amount)]),
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    let owner_id = env.root().sub_account("proxy_owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(condvar_global.clone()),
        on_auth_caller: env.defuse.id().clone(),
        notifier_id: relay.id().clone(),
    };
    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config.clone())
        .await;

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_a_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_b_defuse_id.to_string(),
    ));
    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([proxy_id.clone()], dst_token),
    )
    .build();
    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    let fill_escrow_msg = FillMessageBuilder::new(escrow_params.clone())
        .with_deadline(Deadline::timeout(Duration::from_secs(120)))
        .build();

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(escrow_swap_global.clone()),
        data: escrow_raw_state,
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    let transfer = Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: Amounts::new([(token_a_defuse_id.clone(), swap_amount)].into()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(fund_msg_json).with_state_init(escrow_state_init)),
    };

    // Maker signs and executes transfer intent
    let transfer_payload = maker
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();
    maker
        .simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: Some([2u8; 32]),
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        salt: proxy_msg.salt.unwrap_or_default(),
        msg: Cow::Borrowed(&proxy_msg_json),
    }
    .hash();

    let auth_state = CondVarConfig {
        on_auth_caller: env.defuse.id().clone(),
        notifier_id: relay.id().clone(),
        authorizee: proxy_id.clone(),
        salt: context_hash,
    };
    let condvar_raw_state = CondVarStorage::init_state(auth_state.clone()).unwrap();
    let condvar_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(condvar_global.clone()),
        data: condvar_raw_state,
    });

    let auth_payload = relay
        .sign_defuse_payload_default(
            &env.defuse,
            [AuthCall {
                contract_id: condvar_state_init.derive_account_id(),
                state_init: Some(condvar_state_init),
                msg: String::new(),
                attached_deposit: NearToken::from_yoctonear(0),
                min_gas: None,
            }],
        )
        .await
        .unwrap();
    relay
        .simulate_and_execute_intents(env.defuse.id(), [auth_payload])
        .await
        .unwrap();

    solver
        .mt_transfer_call(
            env.defuse.id(),
            &proxy_id,
            &token_b_defuse_id.to_string(),
            swap_amount,
            None,
            &proxy_msg_json,
        )
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(maker.id(), &token_b_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(&proxy_id, &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Proxy (taker) should have received token-a"
    );
}

/// Test that escrow proxy (as sole taker) can cancel escrow before deadline
#[tokio::test]
async fn test_escrow_proxy_can_cancel_before_deadline() {
    let swap_amount: u128 = 100_000_000;
    let env = Env::builder().build().await;

    let (escrow_swap_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_escrow_swap_global("escrow_swap"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );
    let maker = env.create_named_user("maker").await;

    // Create two tokens: token_a for maker, token_b as dummy dst (required by escrow)
    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([]), // no initial balance needed
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    // Deploy proxy with root as owner (can call cancel_escrow)
    let config = ProxyConfig {
        owner_id: env.root().id().clone(),
        // NOTE: oneshot_condvar_global_id is only used for fill operations.
        // This cancel test doesn't exercise fills, so using escrow_swap_global is acceptable.
        oneshot_condvar_global_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        on_auth_caller: env.defuse.id().clone(),
        notifier_id: env.root().id().clone(), // not used for cancel
    };
    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config)
        .await;

    // Build escrow params with proxy as sole taker
    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_a_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_b_defuse_id.to_string(),
    ));
    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([proxy_id.clone()], dst_token),
    )
    .build();

    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(escrow_swap_global.clone()),
        data: escrow_raw_state,
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    // Fund the escrow
    let transfer = Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: Amounts::new([(token_a_defuse_id.clone(), swap_amount)].into()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(fund_msg_json).with_state_init(escrow_state_init)),
    };

    let transfer_payload = maker
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();
    maker
        .simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    // Verify maker's tokens are in escrow (balance is 0)
    assert_eq!(
        env.defuse
            .mt_balance_of(maker.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        0,
        "Maker should have 0 balance after funding escrow"
    );

    // Root (super_admin) cancels the escrow via proxy
    env.root()
        .cancel_escrow(&proxy_id, &escrow_instance_id, &escrow_params)
        .await
        .unwrap();

    // Verify maker got their tokens back
    assert_eq!(
        env.defuse
            .mt_balance_of(maker.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Maker should have tokens back after cancel"
    );
}
