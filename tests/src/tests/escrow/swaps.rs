//! Parameterized integration tests for escrow swap direct fills.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use crate::tests::defuse::env::Env;
use defuse_core::Deadline;
use defuse_escrow_swap::action::{FillMessageBuilder, FundMessageBuilder};
use defuse_escrow_swap::decimal::UD128;
use defuse_escrow_swap::{OverrideSend, ParamsBuilder};
use defuse_sandbox::{EscrowSwapExt, MtExt, MtViewExt};
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
    /// Override where maker receives dst tokens (defaults to maker)
    maker_receive_dst_to: Option<&'static str>,
    /// Override where taker receives src tokens (defaults to taker)
    taker_receive_src_to: Option<&'static str>,
}

impl Default for EscrowSwapTestCase {
    fn default() -> Self {
        Self {
            price: UD128::ONE,
            maker_balance: 0,
            fills: vec![],
            expected_src_balances: vec![],
            expected_dst_balances: vec![],
            maker_receive_dst_to: None,
            taker_receive_src_to: None,
        }
    }
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
    ..Default::default()
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
    ..Default::default()
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
    ..Default::default()
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
    ..Default::default()
})]
#[case::multiple_takers(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![
        (BOB, 400),      // first taker fills 400
        (CHARLIE, 350),  // second taker fills 350
        (DAVE, 250),     // third taker fills remaining 250
    ],
    // Each taker receives src tokens proportional to their fill
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 400),
        (CHARLIE, 350),
        (DAVE, 250),
    ],
    expected_dst_balances: vec![
        (MAKER, 1_000),  // 400 + 350 + 250
        (BOB, 0),
        (CHARLIE, 0),
        (DAVE, 0),
    ],
    ..Default::default()
})]
#[case::overfunding_excess_refunded(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![(BOB, 1_500)],  // taker sends 1500 but only 1000 needed
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),    // receives all src tokens
    ],
    expected_dst_balances: vec![
        (MAKER, 1_000),  // receives exactly what was needed
        (BOB, 500),      // excess 500 refunded
    ],
    ..Default::default()
})]
#[case::maker_dst_redirect(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![(BOB, 1_000)],
    maker_receive_dst_to: Some(CHARLIE),  // Maker's dst goes to Charlie
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 1_000),
        (CHARLIE, 0),
    ],
    expected_dst_balances: vec![
        (MAKER, 0),       // Maker gets nothing (redirected)
        (BOB, 0),
        (CHARLIE, 1_000), // Charlie receives maker's dst
    ],
    ..Default::default()
})]
#[case::taker_src_redirect(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![(BOB, 1_000)],
    taker_receive_src_to: Some(DAVE),  // Bob's src goes to Dave
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 0),        // Bob gets nothing (redirected)
        (DAVE, 1_000),   // Dave receives Bob's src
    ],
    expected_dst_balances: vec![
        (MAKER, 1_000),
        (BOB, 0),
        (DAVE, 0),
    ],
    ..Default::default()
})]
#[case::both_redirects(EscrowSwapTestCase {
    price: UD128::ONE,
    maker_balance: 1_000,
    fills: vec![(BOB, 1_000)],
    maker_receive_dst_to: Some(CHARLIE),
    taker_receive_src_to: Some(DAVE),
    expected_src_balances: vec![
        (MAKER, 0),
        (BOB, 0),
        (CHARLIE, 0),
        (DAVE, 1_000),    // Dave gets src (redirected from Bob)
    ],
    expected_dst_balances: vec![
        (MAKER, 0),
        (BOB, 0),
        (CHARLIE, 1_000), // Charlie gets dst (redirected from Maker)
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

    // Collect all unique accounts: takers + redirect destinations
    let mut all_accounts: HashSet<&'static str> =
        test_case.fills.iter().map(|(name, _)| *name).collect();
    if let Some(dst_redirect) = test_case.maker_receive_dst_to {
        all_accounts.insert(dst_redirect);
    }
    if let Some(src_redirect) = test_case.taker_receive_src_to {
        all_accounts.insert(src_redirect);
    }

    let accounts: HashMap<_, _> = futures::future::join_all(
        all_accounts
            .iter()
            .map(|name| env.create_named_user(name).map(|acc| (*name, acc))),
    )
    .await
    .into_iter()
    .chain(std::iter::once((MAKER, maker.clone())))
    .collect();

    // Takers for whitelist (only accounts that will fill)
    let unique_takers: HashSet<_> = test_case.fills.iter().map(|(name, _)| *name).collect();
    let dst_token_balances =
        test_case
            .fills
            .iter()
            .fold(HashMap::new(), |mut acc, (name, amount)| {
                *acc.entry(accounts.get(name).unwrap().id().clone())
                    .or_default() += amount;
                acc
            });
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

    let mut params_builder = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        (
            unique_takers
                .iter()
                .map(|name| accounts.get(name).unwrap().id().clone()),
            dst_token,
        ),
    )
    .with_price(test_case.price)
    .with_partial_fills_allowed(test_case.fills.len() > 1);

    // Apply maker receive_dst_to override if specified
    if let Some(dst_redirect) = test_case.maker_receive_dst_to {
        params_builder = params_builder.with_receive_dst_to(OverrideSend {
            receiver_id: Some(accounts.get(dst_redirect).unwrap().id().clone()),
            ..Default::default()
        });
    }

    let escrow_params = params_builder.build();
    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();

    let mut fill_builder = FillMessageBuilder::new(escrow_params.clone())
        .with_deadline(Deadline::timeout(Duration::from_secs(120)));

    // Apply taker receive_src_to override if specified
    if let Some(src_redirect) = test_case.taker_receive_src_to {
        fill_builder = fill_builder.with_receive_src_to(OverrideSend {
            receiver_id: Some(accounts.get(src_redirect).unwrap().id().clone()),
            ..Default::default()
        });
    }

    let fill_escrow_msg = fill_builder.build();

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
        let taker = accounts.get(taker_name).unwrap();
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

    let actual_src_balances: BTreeMap<_, _> =
        futures::future::join_all(test_case.expected_src_balances.iter().map(|(name, _)| {
            let acc = accounts.get(name).unwrap();
            env.defuse
                .mt_balance_of(acc.id(), &src_token_id_str)
                .map(|balance| (*name, balance.unwrap()))
        }))
        .await
        .into_iter()
        .collect();

    let actual_dst_balances: BTreeMap<_, _> =
        futures::future::join_all(test_case.expected_dst_balances.iter().map(|(name, _)| {
            let acc = accounts.get(name).unwrap();
            env.defuse
                .mt_balance_of(acc.id(), &dst_token_id_str)
                .map(|balance| (*name, balance.unwrap()))
        }))
        .await
        .into_iter()
        .collect();

    assert_eq!(
        actual_src_balances, expected_src,
        "src_token balances mismatch"
    );
    assert_eq!(
        actual_dst_balances, expected_dst,
        "dst_token balances mismatch"
    );
}
