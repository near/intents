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
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::{FnCallBuilder, MtExt, MtViewExt};
use defuse_sandbox_ext::{EscrowProxyExt, EscrowSwapAccountExt, TransferAuthAccountExt};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_transfer_auth::TransferAuthContext;
use defuse_transfer_auth::storage::{
    ContractStorage as TransferAuthStorage, StateInit as TransferAuthState,
};
use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{AccountId, Gas, GlobalContractId, NearToken, serde_json::json};

/// Storage deposit for FT token
async fn ft_storage_deposit(
    payer: &defuse_sandbox::SigningAccount,
    token: &AccountId,
    account: &AccountId,
) {
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
    let swap_amount: u128 = 100_000_000; // Fits within ft_deposit_to_root mint limit (1e9)
    let env = Env::builder().build().await;
    let (transfer_auth_global, escrow_swap_global) = futures::join!(
        env.root().deploy_transfer_auth("transfer_auth"),
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
    proxy
        .deploy_escrow_proxy(roles, config.clone())
        .await
        .unwrap();
    let escrow_params = Params {
        maker: maker.id().clone(),
        src_token: TokenId::from(Nep245TokenId::new(
            env.defuse.id().clone(),
            token_a_defuse_id.to_string(),
        )),
        dst_token: TokenId::from(Nep245TokenId::new(
            env.defuse.id().clone(),
            token_b_defuse_id.to_string(),
        )),
        price: UD128::ONE, // 1:1 exchange
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

    let fund_escrow_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fund,
    };

    let fill_escrow_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: UD128::ONE,
            deadline: Deadline::timeout(Duration::from_secs(120)),
            receive_src_to: defuse_escrow_swap::OverrideSend::default()
                .receiver_id(solver.id().clone()),
        }),
    };

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init =
        StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(escrow_swap_global.clone()),
            data: escrow_raw_state,
        });
    let escorow_instance_id = escrow_state_init.derive_account_id();

    let transfer = Transfer {
        receiver_id: escorow_instance_id.clone(),
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
        receiver_id: escorow_instance_id.clone(),
        salt: [2u8; 32],
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

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
    let transfer_auth_raw_state = TransferAuthStorage::init_state(auth_state.clone()).unwrap();
    let transfer_auth_state_init =
        StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(transfer_auth_global.clone()),
            data: transfer_auth_raw_state,
        });

    let auth_payload = relay
        .sign_defuse_payload_default(
            &env.defuse,
            [AuthCall {
                contract_id: transfer_auth_state_init.derive_account_id(),
                state_init: Some(transfer_auth_state_init),
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
            .mt_balance_of(solver.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Solver should have received token-a"
    );
}
