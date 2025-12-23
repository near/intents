//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::{Account, FnCallBuilder, MtExt, MtViewExt};
use defuse_sandbox_ext::{
    DefuseAccountExt, EscrowProxyExt, EscrowSwapAccountExt, TransferAuthAccountExt,
    derive_escrow_swap_account_id, derive_transfer_auth_account_id, public_key_from_secret,
};
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_transfer_auth::TransferAuthContext;
use defuse_transfer_auth::storage::StateInit as TransferAuthState;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, GlobalContractId, NearToken, serde_json::json};

/// Hardcoded test private key for relay (32 bytes) - FOR TESTING ONLY
const PRIVATE_KEY_RELAY: [u8; 32] = [1u8; 32];

/// Storage deposit for FT token
async fn ft_storage_deposit(payer: &defuse_sandbox::SigningAccount, token: &AccountId, account: &AccountId) {
    payer
        .tx(token.clone())
        .function_call(
            FnCallBuilder::new("storage_deposit")
                .json_args(json!({ "account_id": account }))
                .with_gas(Gas::from_tgas(50))
                .with_deposit(NearToken::from_millinear(50)),
        )
        .await
        .ok();
}

/// Test full escrow swap flow with proxy authorization
#[tokio::test]
async fn test_escrow_swap_with_proxy_full_flow() {
    let env = Env::builder().build().await;

    // Deploy global contracts in parallel
    let (transfer_auth_global, escrow_swap_global) = futures::join!(
        env.root().deploy_transfer_auth("transfer_auth"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
    );

    // Create accounts in parallel
    let (maker, solver, relay, proxy) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    // Deploy escrow-proxy
    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(transfer_auth_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };
    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();

    // 2. Deploy and setup tokens
    let swap_amount: u128 = 100_000_000; // Fits within ft_deposit_to_root mint limit (1e9)

    // Deploy tokens with initial balances using Env helper (deposits to env.defuse)
    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), swap_amount)]),
    );
    let (token_a, token_a_defuse_id) = token_a_result.unwrap();
    let (token_b, token_b_defuse_id) = token_b_result.unwrap();
    let token_a_id = token_a.id().clone();
    let token_b_id = token_b.id().clone();

    // Escrow-swap uses Nep245 format token IDs (nep245:<defuse>:<inner_token>)
    let token_a_escrow_id =
        TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), token_a_defuse_id.to_string()));
    let token_b_escrow_id =
        TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), token_b_defuse_id.to_string()));

    // 3. Create escrow parameters (use escrow token IDs - Nep245 format)
    let escrow_params = Params {
        maker: maker.id().clone(),
        src_token: token_a_escrow_id.clone(), // What maker offers (Nep245 format)
        dst_token: token_b_escrow_id.clone(), // What maker wants (Nep245 format)
        price: UD128::ONE,                    // 1:1 exchange
        deadline: Deadline::timeout(Duration::from_secs(360)),
        partial_fills_allowed: false,
        refund_src_to: defuse_escrow_swap::OverrideSend::default(),
        receive_dst_to: defuse_escrow_swap::OverrideSend::default(),
        taker_whitelist: [proxy.id().clone()].into(), // Anyone can fill
        protocol_fees: None,
        integrator_fees: BTreeMap::new(),
        auth_caller: None,
        salt: [1u8; 32],
    };

    // Derive escrow instance ID (without deploying) to check existence
    let escrow_instance_id = derive_escrow_swap_account_id(&escrow_swap_global, &escrow_params);
    let escrow_instance_account =
        Account::new(escrow_instance_id.clone(), env.root().network_config().clone());

    // Verify escrow-swap instance does NOT exist before maker's fund
    assert!(
        escrow_instance_account.view().await.is_err(),
        "Escrow-swap instance should NOT exist before maker's fund"
    );

    // 4. Fund escrow via Transfer intent with state_init
    // This deploys the escrow instance atomically with the token transfer
    let fund_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fund,
    };
    let fund_msg_json = serde_json::to_string(&fund_msg).unwrap();

    // Build state_init for escrow-swap instance
    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init =
        near_sdk::state_init::StateInit::V1(near_sdk::state_init::StateInitV1 {
            code: near_sdk::GlobalContractId::AccountId(escrow_swap_global.clone()),
            data: escrow_raw_state,
        });

    // Build Transfer intent with notification containing state_init
    let transfer = defuse_core::intents::tokens::Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: defuse_core::amounts::Amounts::new(
            [(token_a_defuse_id.clone(), swap_amount)].into(),
        ),
        memo: None,
        notification: Some(
            defuse_core::intents::tokens::NotifyOnTransfer::new(fund_msg_json)
                .with_state_init(escrow_state_init),
        ),
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

    // Verify escrow-swap instance EXISTS after maker's fund (state_init deployed it)
    assert!(
        escrow_instance_account.view().await.is_ok(),
        "Escrow-swap instance should exist after maker's fund"
    );

    // Verify escrow received maker's tokens
    let escrow_token_a_balance = env
        .defuse
        .mt_balance_of(&escrow_instance_id, &token_a_defuse_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_token_a_balance, swap_amount,
        "Escrow should have token-a"
    );

    // 5. Solver fills via proxy with relay authorization
    // Build the inner escrow-swap fill message
    // NOTE: receive_src_to must be set to solver's account because escrow-swap
    // will use sender_id (proxy) as default recipient. Since proxy forwards tokens,
    // we need to explicitly override where maker's src tokens go.
    let inner_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: UD128::ONE,
            deadline: Deadline::timeout(Duration::from_secs(120)),
            receive_src_to: defuse_escrow_swap::OverrideSend::default()
                .receiver_id(solver.id().clone()),
        }),
    };
    let inner_msg_json = serde_json::to_string(&inner_msg).unwrap();

    // Build proxy message that wraps the inner escrow message
    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [2u8; 32],
        msg: inner_msg_json,
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    // Derive transfer-auth instance from context hash
    // The hash is computed from TransferAuthContext matching how proxy computes it
    let context_hash = TransferAuthContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        salt: proxy_msg.salt,
        msg: Cow::Borrowed(&proxy_msg_json),
    }
    .hash();

    let auth_state = TransferAuthState {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: env.defuse.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let transfer_auth_instance_id = derive_transfer_auth_account_id(
        &GlobalContractId::AccountId(transfer_auth_global.clone()),
        &auth_state,
    );

    // Create Account wrapper for transfer-auth instance to check existence
    let transfer_auth_instance_account = Account::new(
        transfer_auth_instance_id.clone(),
        env.root().network_config().clone(),
    );

    // Verify transfer-auth instance does NOT exist before on_auth call
    assert!(
        transfer_auth_instance_account.view().await.is_err(),
        "Transfer-auth instance should NOT exist before on_auth call"
    );


    relay
        .defuse_add_public_key(&env.defuse, public_key_from_secret(&PRIVATE_KEY_RELAY))
        .await
        .unwrap();
    relay
        .execute_auth_call_intent(
            &env.defuse,
            &transfer_auth_global,
            &auth_state,
            &PRIVATE_KEY_RELAY,
            [0u8; 32],
        )
        .await;

    // Verify transfer-auth instance EXISTS after on_auth call (state_init deployed it)
    assert!(
        transfer_auth_instance_account.view().await.is_ok(),
        "Transfer-auth instance should exist after on_auth call"
    );

    // Step 2: Solver sends tokens to proxy - proxy will query transfer-auth for authorization
    let transfer_result = solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &token_b_defuse_id.to_string(),
            swap_amount,
            None,
            &proxy_msg_json,
        )
        .await;

    let used_amounts = transfer_result.unwrap();

    // 6. Verify final state
    // When authorized, tokens should be forwarded and swap completed
    assert_eq!(
        used_amounts,
        swap_amount,
        "Used amount should equal transferred amount when authorized"
    );

    // Maker should have received token-b
    let maker_token_b_balance = env
        .defuse
        .mt_balance_of(maker.id(), &token_b_defuse_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        maker_token_b_balance, swap_amount,
        "Maker should have received token-b"
    );

    // Solver should have received token-a
    let solver_token_a_balance = env
        .defuse
        .mt_balance_of(solver.id(), &token_a_defuse_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        solver_token_a_balance, swap_amount,
        "Solver should have received token-a"
    );

    // Escrow should be empty
    let escrow_token_a_final = env
        .defuse
        .mt_balance_of(&escrow_instance_id, &token_a_defuse_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_token_a_final, 0,
        "Escrow should have no token-a remaining"
    );

    let escrow_token_b_final = env
        .defuse
        .mt_balance_of(&escrow_instance_id, &token_b_defuse_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_token_b_final, 0,
        "Escrow should have no token-b remaining"
    );
}
