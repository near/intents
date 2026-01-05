//! Integration tests for escrow swap fees (protocol fees, integrator fees, surplus fees).

use std::time::Duration;

use crate::tests::defuse::env::Env;
use defuse_deadline::Deadline;
use defuse_escrow_swap::decimal::UD128;
use defuse_escrow_swap::{ParamsBuilder, Pips, ProtocolFees};
use defuse_sandbox::{MtExt, MtViewExt};
use defuse_sandbox_ext::EscrowSwapAccountExt;
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;

/// Test escrow with 1% protocol fee
#[tokio::test]
async fn test_escrow_with_protocol_fee() {
    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;
    let fee_collector = env.create_named_user("fee_collector").await;

    let amount = 1_000_u128;

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), amount)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), amount)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), src_token_id.to_string()));
    let dst_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), dst_token_id.to_string()));

    let (escrow_params, fund_msg, fill_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_protocol_fees(ProtocolFees {
        fee: Pips::from_percent(1).unwrap(),  // 1% fee
        surplus: Pips::ZERO,
        collector: fee_collector.id().clone(),
    })
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    let escrow_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global, &escrow_params)
        .await;

    // Fund escrow
    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &src_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Fill escrow
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Assert: taker gets all 1000 src
    let taker_src = env.defuse.mt_balance_of(taker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(taker_src, 1_000, "taker should receive all src");

    // Assert: maker gets 990 dst (1000 - 1% fee)
    let maker_dst = env.defuse.mt_balance_of(maker.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(maker_dst, 990, "maker should receive 990 dst (1000 - 1% fee)");

    // Assert: fee_collector gets 10 dst (1% of 1000)
    let collector_dst = env.defuse.mt_balance_of(fee_collector.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(collector_dst, 10, "fee_collector should receive 10 dst (1% fee)");
}

/// Test escrow with 2% integrator fee
#[tokio::test]
async fn test_escrow_with_integrator_fee() {
    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;
    let integrator = env.create_named_user("integrator").await;

    let amount = 1_000_u128;

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), amount)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), amount)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), src_token_id.to_string()));
    let dst_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), dst_token_id.to_string()));

    let (escrow_params, fund_msg, fill_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_integrator_fee(integrator.id().clone(), Pips::from_percent(2).unwrap())  // 2% fee
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    let escrow_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global, &escrow_params)
        .await;

    // Fund escrow
    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &src_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Fill escrow
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Assert: taker gets all 1000 src
    let taker_src = env.defuse.mt_balance_of(taker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(taker_src, 1_000, "taker should receive all src");

    // Assert: maker gets 980 dst (1000 - 2% fee)
    let maker_dst = env.defuse.mt_balance_of(maker.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(maker_dst, 980, "maker should receive 980 dst (1000 - 2% fee)");

    // Assert: integrator gets 20 dst (2% of 1000)
    let integrator_dst = env.defuse.mt_balance_of(integrator.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(integrator_dst, 20, "integrator should receive 20 dst (2% fee)");
}

/// Test escrow with combined protocol (1%) and integrator (2%) fees
#[tokio::test]
async fn test_escrow_with_combined_fees() {
    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;
    let protocol_collector = env.create_named_user("protocol").await;
    let integrator = env.create_named_user("integrator").await;

    let amount = 1_000_u128;

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), amount)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), amount)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), src_token_id.to_string()));
    let dst_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), dst_token_id.to_string()));

    let (escrow_params, fund_msg, fill_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_protocol_fees(ProtocolFees {
        fee: Pips::from_percent(1).unwrap(),  // 1% protocol fee
        surplus: Pips::ZERO,
        collector: protocol_collector.id().clone(),
    })
    .with_integrator_fee(integrator.id().clone(), Pips::from_percent(2).unwrap())  // 2% integrator fee
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    let escrow_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global, &escrow_params)
        .await;

    // Fund escrow
    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &src_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Fill escrow
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            amount,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Assert: taker gets all 1000 src
    let taker_src = env.defuse.mt_balance_of(taker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(taker_src, 1_000, "taker should receive all src");

    // Assert: maker gets 970 dst (1000 - 3% total fees)
    let maker_dst = env.defuse.mt_balance_of(maker.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(maker_dst, 970, "maker should receive 970 dst (1000 - 3% fees)");

    // Assert: protocol_collector gets 10 dst (1% of 1000)
    let protocol_dst = env.defuse.mt_balance_of(protocol_collector.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(protocol_dst, 10, "protocol should receive 10 dst (1% fee)");

    // Assert: integrator gets 20 dst (2% of 1000)
    let integrator_dst = env.defuse.mt_balance_of(integrator.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(integrator_dst, 20, "integrator should receive 20 dst (2% fee)");
}

/// Test escrow with surplus fee (captures value when taker pays above maker's price)
/// Maker: 1000 src at price 1.0 (minimum 1000 dst)
/// Taker: fills at price 2.0 (pays 2000 dst for 1000 src)
/// Surplus = 2000 - 1000 = 1000 dst, 50% fee = 500 dst
#[tokio::test]
async fn test_escrow_with_surplus_fee() {
    use defuse_escrow_swap::action::{FillAction, TransferAction, TransferMessage};
    use defuse_escrow_swap::OverrideSend;

    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;
    let fee_collector = env.create_named_user("fee_collector").await;

    let maker_src = 1_000_u128;
    let taker_dst = 2_000_u128;  // Taker pays at price 2.0

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), maker_src)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), taker_dst)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), src_token_id.to_string()));
    let dst_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), dst_token_id.to_string()));

    // Maker price 1.0, surplus fee 50% (no base fee)
    let (escrow_params, fund_msg, _) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_protocol_fees(ProtocolFees {
        fee: Pips::ZERO,                          // no base fee
        surplus: Pips::from_percent(50).unwrap(), // 50% surplus fee
        collector: fee_collector.id().clone(),
    })
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    // Custom fill message with price 2.0 (higher than maker's 1.0)
    let fill_msg = TransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: UD128::from(2),  // Taker offers price 2.0
            deadline: Deadline::timeout(Duration::from_secs(120)),
            receive_src_to: OverrideSend::default(),
        }),
    };

    let escrow_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global, &escrow_params)
        .await;

    // Fund escrow
    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &src_token_id.to_string(),
            maker_src,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Fill escrow at higher price
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            taker_dst,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Assert: taker gets all 1000 src
    let taker_src_bal = env.defuse.mt_balance_of(taker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(taker_src_bal, 1_000, "taker should receive 1000 src");

    // Assert: maker gets 1500 dst (2000 - 500 surplus fee)
    let maker_dst_bal = env.defuse.mt_balance_of(maker.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(maker_dst_bal, 1_500, "maker should receive 1500 dst (2000 - 500 surplus fee)");

    // Assert: fee_collector gets 500 dst (50% of 1000 surplus)
    let collector_dst = env.defuse.mt_balance_of(fee_collector.id(), &dst_token_id.to_string()).await.unwrap();
    assert_eq!(collector_dst, 500, "fee_collector should receive 500 dst (50% of surplus)");
}
