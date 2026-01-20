//! Gas benchmark test for escrow swap with proxy flow.
//!
//! Tests the full proxy → condvar → escrow path to measure total gas consumption.
//! Uses worst-case scenario with protocol fees, integrator fees, and surplus fees.

use std::borrow::Cow;
use std::time::Duration;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{
    MT_ON_TRANSFER_GAS, ProxyConfig, TransferMessage as ProxyTransferMessage,
};
use defuse_escrow_swap::ParamsBuilder;
use defuse_escrow_swap::action::{FillAction, FundMessageBuilder, TransferAction, TransferMessage};
use defuse_escrow_swap::decimal::UD128;
use defuse_escrow_swap::{OverrideSend, Pips, ProtocolFees};
use defuse_oneshot_condvar::CondVarContext;
use defuse_oneshot_condvar::storage::{
    ContractStorage as CondVarStorage, StateInit as CondVarState,
};
use defuse_sandbox::{MtExt, MtViewExt};
use defuse_sandbox_ext::{EscrowProxyExt, EscrowSwapAccountExt, OneshotCondVarAccountExt};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;

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
    let (condvar_global, escrow_swap_global) = futures::join!(
        env.root().deploy_oneshot_condvar("oneshot_condvar"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
    );
    let (maker, solver, relay, proxy, fee_collector, integrator) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
        env.create_named_user("fee_collector"),
        env.create_named_user("integrator"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), solver_amount)]),
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
    // Configure escrow with all fee types for worst-case gas scenario
    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([proxy.id().clone()], dst_token),
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

    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [2u8; 32],
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(solver_amount)]), // 2x for price 2.0
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
            proxy.id(),
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
            .mt_balance_of(proxy.id(), &token_a_defuse_id.to_string())
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

    // Assert gas consumed <= MT_ON_TRANSFER_GAS
    assert!(
        MT_ON_TRANSFER_GAS >= total_gas,
        "MT_ON_TRANSFER_GAS ({MT_ON_TRANSFER_GAS:?}) should be >= actual ({total_gas:?})",
    );
}
