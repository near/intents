//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;
use super::EscrowExt;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::ParamsBuilder;
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::{MtExt, MtViewExt};
use defuse_sandbox_ext::{EscrowProxyExt, EscrowSwapAccountExt, TransferAuthAccountExt};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_transfer_auth::TransferAuthContext;
use defuse_transfer_auth::storage::{
    ContractStorage as TransferAuthStorage, StateInit as TransferAuthState,
};

use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{GlobalContractId, NearToken};
use rstest::rstest;

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

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_a_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_b_defuse_id.to_string(),
    ));
    let (escrow_params, fund_escrow_msg, fill_escrow_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([proxy.id().clone()], dst_token),
    )
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = StateInit::V1(StateInitV1 {
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
    let transfer_auth_state_init = StateInit::V1(StateInitV1 {
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
            .mt_balance_of(proxy.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Proxy (taker) should have received token-a"
    );
}

// User name constants for parameterized tests
const MAKER: &str = "alice";
const BOB: &str = "bob";
const CHARLIE: &str = "charlie";
const DAVE: &str = "dave";

/// Test case configuration for escrow swap
#[derive(Debug, Clone)]
struct EscrowSwapTestCase {
    price: UD128,
    /// Maker (ALICE) initial `src_token` balance
    maker_balance: u128,
    /// Fills: `(account_name, fill_amount)` - `dst_token` minted per account
    fills: Vec<(&'static str, u128)>,
    /// Expected `src_token` balances after fills (dust may remain in escrow)
    expected_src_balances: Vec<(&'static str, u128)>,
    /// Expected `dst_token` balances after fills
    expected_dst_balances: Vec<(&'static str, u128)>,
}

#[rstest]
#[case::simple_1_to_1_swap(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 100_000_000,
    fills: vec![(BOB, 100_000_000)],
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 100_000_000),
    ],
    expected_dst_balances: vec![
        (MAKER, 100_000_000),
        (BOB, 0),
    ],
})]
#[case::price_ratio_1_to_2(EscrowSwapTestCase {
    price: UD128::from(2),
    maker_balance: 1_000,
    fills: vec![(BOB, 2_000)],
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),
    ],
    expected_dst_balances: vec![
        (MAKER, 2_000),
        (BOB, 0),
    ],
})]
#[case::price_ratio_1_to_3(EscrowSwapTestCase {
    price: UD128::from(3),
    maker_balance: 1_000,
    fills: vec![(BOB, 3_000)],  // 3000/3 = 1000 (exact)
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),
    ],
    expected_dst_balances: vec![
        (MAKER, 3_000),
        (BOB, 0),
    ],
})]
#[case::fractional_price_1_333333(EscrowSwapTestCase {
    price: "1.333333".parse().unwrap(),  // ≈4/3
    maker_balance: 1_000,
    fills: vec![(BOB, 1_334)],  // 1334/1.333333 = 1000.5... rounds to 1000
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),
    ],
    expected_dst_balances: vec![
        (MAKER, 1_334),
        (BOB, 0),
    ],
})]
#[case::multiple_takers(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![
        (BOB, 400),      // first taker fills 400
        (CHARLIE, 350),  // second taker fills 350
        (DAVE, 250),     // third taker fills remaining 250
    ],
    // Note: fill_msg has receive_src_to pointing to first taker (BOB),
    // so all src tokens go to BOB regardless of who fills
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),    // receives all src (400+350+250)
        (CHARLIE, 0),
        (DAVE, 0),
    ],
    expected_dst_balances: vec![
        (MAKER, 1_000),  // 400 + 350 + 250
        (BOB, 0),
        (CHARLIE, 0),
        (DAVE, 0),
    ],
})]
#[tokio::test]
async fn test_escrow_swap_direct_fill(#[case] test_case: EscrowSwapTestCase) {
    use futures::FutureExt;

    // Arrange
    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;
    let maker = env.create_named_user(MAKER).await;

    let unique_takers: HashSet<_> = test_case.fills.iter().map(|(name, _)| name).collect();
    let takers: HashMap<_, _> = futures::future::join_all(
        unique_takers
            .iter()
            .map(|name| env.create_named_user(name).map(|taker| (*name, taker))),
    )
    .await
    .into_iter()
    .chain(std::iter::once((&MAKER, maker.clone())))
    .collect();
    let dst_token_balances = test_case.fills.iter().fold(
        HashMap::new(),
        |mut acc, (name, amount)| {
            *acc.entry(takers.get(name).unwrap().id().clone())
                .or_default() += amount;
            acc
        },
    );
    let ((_, src_token_defuse_id), (_, dst_token_defuse_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), test_case.maker_balance)]),
        env.create_mt_token_with_initial_balances(dst_token_balances),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        src_token_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        dst_token_defuse_id.to_string(),
    ));

    let (escrow_params, fund_escrow_msg, fill_escrow_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        (unique_takers.iter().map(|name| takers.get(name).unwrap().id().clone()), dst_token),
    )
    .with_price(test_case.price)
    .with_partial_fills_allowed(test_case.fills.len() > 1)
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(360)),
        Deadline::timeout(Duration::from_secs(120)),
    );

    // Act
    let escrow_instance_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global.clone(), &escrow_params)
        .await;

    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_instance_id,
            &src_token_defuse_id.to_string(),
            test_case.maker_balance,
            None,
            &serde_json::to_string(&fund_escrow_msg).unwrap(),
        )
        .await
        .unwrap();

    for (taker_name, fill_amount) in &test_case.fills {
        let taker = takers.get(taker_name).unwrap();
        taker
            .mt_transfer_call(
                env.defuse.id(),
                &escrow_instance_id,
                &dst_token_defuse_id.to_string(),
                *fill_amount,
                None,
                &serde_json::to_string(&fill_escrow_msg).unwrap(),
            )
            .await
            .unwrap();
    }

    // Assert balances after fills
    let expected_src: BTreeMap<_, _> = test_case.expected_src_balances.iter().copied().collect();
    let expected_dst: BTreeMap<_, _> = test_case.expected_dst_balances.iter().copied().collect();

    let src_token_id_str = src_token_defuse_id.to_string();
    let dst_token_id_str = dst_token_defuse_id.to_string();

    let actual_src_balances: BTreeMap<_, _> = futures::future::join_all(test_case.expected_src_balances.iter().map(|(name, _)| {
                let acc = takers.get(name).unwrap();
                env.defuse
                    .mt_balance_of(acc.id(), &src_token_id_str).map(|balance| (*name, balance.unwrap()))
    })).await.into_iter().collect();

    let actual_dst_balances: BTreeMap<_, _> = futures::future::join_all(test_case.expected_dst_balances.iter().map(|(name, _)| {
                let acc = takers.get(name).unwrap();
                env.defuse
                    .mt_balance_of(acc.id(), &dst_token_id_str).map(|balance| (*name, balance.unwrap()))
    })).await.into_iter().collect();

    assert_eq!(actual_src_balances, expected_src, "src_token balances mismatch");
    assert_eq!(actual_dst_balances, expected_dst, "dst_token balances mismatch");
}

/// Test partial fill with dust: verifies remaining funds returned to maker after timeout
#[tokio::test]
async fn test_partial_fill_funds_returned_after_timeout() {
    use super::EscrowExt;

    let env = Env::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;

    // Price 0.333333: taker pays 0.333333 dst per 1 src (gets ~3 src per 1 dst)
    // Maker deposits 1000 src, taker fills with 166 dst (~50%)
    // floor(166 / 0.333333) = 498 src to taker
    // 1000 - 498 = 502 src remaining (partial + rounding dust)
    let maker_balance = 1_000_u128;
    let fill_amount = 166_u128;
    let price: UD128 = "0.333333".parse().unwrap();
    let expected_taker_src = 498_u128;  // floor(166 / 0.333333)
    let expected_maker_refund = maker_balance - expected_taker_src;  // 502

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), maker_balance)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), fill_amount)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), src_token_id.to_string()));
    let dst_token = TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), dst_token_id.to_string()));

    let (escrow_params, fund_msg, fill_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_price(price)
    .with_partial_fills_allowed(true)
    .build_with_messages(
        Deadline::timeout(Duration::from_secs(6)),
        Deadline::timeout(Duration::from_secs(5)),
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
            maker_balance,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Partial fill (~50%)
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            fill_amount,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Verify taker received expected src
    let taker_src = env.defuse.mt_balance_of(taker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(taker_src, expected_taker_src, "taker should have floor(166/0.333333) = 498");

    // Maker has 0 before close (remaining in escrow)
    let maker_src_before = env.defuse.mt_balance_of(maker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(maker_src_before, 0, "maker src should be 0 before close");

    // Wait for deadline to expire
    tokio::time::sleep(Duration::from_secs(7)).await;

    // Close escrow - remaining funds return to maker
    maker.es_close(&escrow_id, &escrow_params).await.unwrap();

    // Verify maker received remaining src (partial fill remainder + rounding dust)
    let maker_src_after = env.defuse.mt_balance_of(maker.id(), &src_token_id.to_string()).await.unwrap();
    assert_eq!(maker_src_after, expected_maker_refund, "maker should receive 502 src after close");
}
