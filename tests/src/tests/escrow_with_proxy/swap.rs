//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::time::Duration;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_core::Deadline;
use defuse_escrow_proxy::{ProxyConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::ParamsBuilder;
use defuse_escrow_swap::action::{FillMessageBuilder, FundMessageBuilder};
use defuse_oneshot_condvar::CondVarContext;
use defuse_oneshot_condvar::storage::{
    ContractStorage as CondVarStorage, StateInit as CondVarState,
};
use defuse_sandbox::{EscrowProxyExt, EscrowSwapExt, MtExt, MtViewExt, OneshotCondVarExt};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;

use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{GlobalContractId, NearToken};

/// Test full escrow swap flow with proxy authorization
#[tokio::test]
async fn test_escrow_swap_with_proxy_full_flow() {
    let swap_amount: u128 = 100_000_000; // Fits within ft_deposit_to_root mint limit (1e9)
    let env = Env::builder().build().await;
    let (condvar_global, escrow_swap_global) = futures::join!(
        env.root().deploy_oneshot_condvar("oneshot_condvar"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
    );
    let (maker, solver, relay, proxy) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), swap_amount)]),
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    let config = ProxyConfig {
        owner: proxy.id().clone(),
        per_fill_contract_id: GlobalContractId::AccountId(condvar_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };
    proxy.deploy_escrow_proxy(config.clone()).await.unwrap();

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
        ([proxy.id().clone()], dst_token),
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
        salt: [2u8; 32],
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        salt: proxy_msg.salt,
        msg: Cow::Borrowed(&proxy_msg_json),
    }
    .hash();

    let auth_state = CondVarState {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: env.defuse.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
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
            proxy.id(),
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
            .mt_balance_of(proxy.id(), &token_a_defuse_id.to_string())
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

    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;
    let (maker, proxy) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("proxy"),
    );

    // Create two tokens: token_a for maker, token_b as dummy dst (required by escrow)
    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([]), // no initial balance needed
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    // Deploy proxy with root as owner (can call cancel_escrow)
    let config = ProxyConfig {
        owner: env.root().id().clone(),
        // NOTE: per_fill_contract_id is only used for fill operations.
        // This cancel test doesn't exercise fills, so using escrow_swap_global is acceptable.
        per_fill_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: env.root().id().clone(), // not used for cancel
    };
    proxy.deploy_escrow_proxy(config).await.unwrap();

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
        ([proxy.id().clone()], dst_token),
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
        .cancel_escrow(proxy.id(), &escrow_params)
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
