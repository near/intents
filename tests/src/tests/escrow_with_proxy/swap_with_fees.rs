//! Gas benchmark test for escrow swap with proxy flow.
//!
//! Tests the full proxy → condvar → escrow path to measure total gas consumption.
//! Uses worst-case scenario with protocol fees, integrator fees, and surplus fees.

use std::borrow::Cow;
use std::time::Duration;

use crate::env::Env;
use crate::extensions::condvar::OneshotCondVarExt;
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use crate::extensions::escrow::EscrowSwapExt;
use crate::extensions::escrow_proxy::EscrowProxyExt;
use crate::utils::escrow_builders::FundMessageBuilder;
use crate::utils::escrow_builders::ParamsBuilder;
use defuse_core::Deadline;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_core::token_id::TokenId;
use defuse_core::token_id::nep245::Nep245TokenId;
use defuse_escrow_proxy::CondVarContext;
use defuse_escrow_proxy::{ForwardRequest as ProxyForwardRequest, ProxyConfig};
use defuse_escrow_swap::action::{FillAction, TransferAction, TransferMessage};
use defuse_escrow_swap::decimal::UD128;
use defuse_escrow_swap::{OverrideSend, Pips, ProtocolFees};
use defuse_oneshot_condvar::storage::{Config as CondVarConfig, ContractStorage as CondVarStorage};
use defuse_sandbox::{MtExt, MtViewExt};
use near_sdk::serde_json;

use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{Gas, GlobalContractId, NearToken};

/// Benchmark test for proxy fill gas consumption (worst case with all fees).
/// Tests the full flow: solver → proxy → condvar → escrow with:
/// - Protocol fee (1%)
/// - Integrator fee (2%)
/// - Surplus fee (50%) - triggered by higher fill price
#[tokio::test]
async fn test_proxy_fill_gas_benchmark() {
    let swap_amount: u128 = 100_000_000;
    let solver_amount: u128 = swap_amount * 2; // 2x for price 2.0 fill
    let env = Env::builder().build().await;
    let (condvar_global, escrow_swap_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("oneshot_condvar"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );
    let (maker, solver, relay, fee_collector, integrator) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("fee_collector"),
        env.create_named_user("integrator"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), solver_amount)]),
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
    // Configure escrow with all fee types for worst-case gas scenario
    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([proxy_id.clone()], dst_token),
    )
    .with_protocol_fees(ProtocolFees {
        fee: Pips::from_percent(1).unwrap(),      // 1% protocol fee
        surplus: Pips::from_percent(50).unwrap(), // 50% surplus fee
        collector: fee_collector.id().clone(),
    })
    .with_integrator_fee(integrator.id().clone(), Pips::from_percent(2).unwrap())
    .build();
    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    // Fill at price 2.0 to trigger surplus fee (maker price is 1.0)
    let fill_escrow_msg = TransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: UD128::from(2), // 2x maker's price
            deadline: Deadline::timeout(Duration::from_secs(120)),
            receive_src_to: OverrideSend::default(),
        }),
    };

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

    // Maker signs and executes transfer intent to fund escrow
    let transfer_payload = maker
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();
    maker
        .simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    let proxy_msg = ProxyForwardRequest {
        receiver_id: escrow_instance_id.clone(),
        salt: Some([2u8; 32]),
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![TokenId::from(Nep245TokenId::new(
            env.defuse.id().clone(),
            token_b_defuse_id.to_string(),
        ))
        .to_string()]),
        amounts: Cow::Owned(vec![U128(solver_amount)]), // 2x for price 2.0
        receiver_id: Cow::Borrowed(proxy_msg.receiver_id.as_ref()),
        msg: Cow::Borrowed(&proxy_msg.msg),
    }
    .hash();

    let auth_state = CondVarConfig {
        on_auth_caller: env.defuse.id().clone(),
        notifier_id: relay.id().clone(),
        waiter: proxy_id.clone(),
        salt: context_hash,
    };
    let condvar_raw_state = CondVarStorage::init_state(auth_state.clone()).unwrap();
    let condvar_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(condvar_global.clone()),
        data: condvar_raw_state,
    });

    // Relay signs authorization
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

    // Measure the solver's mt_transfer_call to proxy - this is the key operation
    let result = solver
        .mt_transfer_call_exec(
            env.defuse.id(),
            &proxy_id,
            &token_b_defuse_id.to_string(),
            solver_amount, // 2x for price 2.0
            None,
            &proxy_msg_json,
        )
        .await
        .unwrap();

    let total_gas = Gas::from_gas(result.total_gas_burnt.as_gas());

    eprintln!("Proxy fill total gas consumed (worst case with fees): {total_gas:?}");

    // Verify swap completed successfully - proxy received src tokens
    assert!(
        env.defuse
            .mt_balance_of(&proxy_id, &token_a_defuse_id.to_string())
            .await
            .unwrap()
            > 0,
        "Proxy (taker) should have received token-a"
    );

    // Verify maker received dst tokens (minus fees)
    assert!(
        env.defuse
            .mt_balance_of(maker.id(), &token_b_defuse_id.to_string())
            .await
            .unwrap()
            > 0,
        "Maker should have received token-b"
    );
}
