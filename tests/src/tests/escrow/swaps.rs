//! Parameterized integration tests for escrow swap direct fills.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use crate::tests::defuse::env::Env;
use defuse_deadline::Deadline;
use defuse_escrow_swap::ParamsBuilder;
use defuse_escrow_swap::action::{FillMessageBuilder, FundMessageBuilder};
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::{MtExt, MtViewExt};
use defuse_sandbox_ext::EscrowSwapAccountExt;
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use rstest::rstest;

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

    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        (unique_takers.iter().map(|name| takers.get(name).unwrap().id().clone()), dst_token),
    )
    .with_price(test_case.price)
    .with_partial_fills_allowed(test_case.fills.len() > 1)
    .build();
    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    let fill_escrow_msg = FillMessageBuilder::new(escrow_params.clone())
        .with_deadline(Deadline::timeout(Duration::from_secs(120)))
        .build();

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
