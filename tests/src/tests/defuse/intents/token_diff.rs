use std::{collections::BTreeMap, time::Duration};

use defuse::core::{
    fees::Pips,
    intents::{
        token_diff::{TokenDeltas, TokenDiff},
        DefuseIntents,
    },
    payload::multi::MultiPayload,
    tokens::TokenId,
    Deadline,
};
use near_sdk::AccountId;
use near_workspaces::Account;
use randomness::{make_true_rng, Rng};
use rstest::rstest;

use crate::{
    tests::defuse::{env::Env, DefuseSigner},
    utils::mt::MtExt,
};

use super::ExecuteIntentsExt;

#[rstest]
#[tokio::test]
async fn test_swap_p2p(#[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips) {
    let env = Env::builder().fee(fee).build().await;

    let ft1_token_id = TokenId::Nep141(env.ft1.clone());
    let ft2_token_id = TokenId::Nep141(env.ft2.clone());

    test_ft_diffs(
        &env,
        [
            AccountFtDiff {
                account: &env.user1,
                init_balances: [(&env.ft1, 100)].into_iter().collect(),
                diff: [TokenDeltas::default()
                    .with_add_deltas([
                        (ft1_token_id.clone(), -100),
                        (
                            ft2_token_id.clone(),
                            TokenDiff::closure_delta(&ft2_token_id, -200, fee).unwrap(),
                        ),
                    ])
                    .unwrap()]
                .into(),
                result_balances: [(
                    &env.ft2,
                    TokenDiff::closure_delta(&ft2_token_id, -200, fee).unwrap(),
                )]
                .into_iter()
                .collect(),
            },
            AccountFtDiff {
                account: &env.user2,
                init_balances: [(&env.ft2, 200)].into_iter().collect(),
                diff: [TokenDeltas::default()
                    .with_add_deltas([
                        (
                            ft1_token_id.clone(),
                            TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                        ),
                        (ft2_token_id.clone(), -200),
                    ])
                    .unwrap()]
                .into(),
                result_balances: [(
                    &env.ft1,
                    TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                )]
                .into_iter()
                .collect(),
            },
        ]
        .into(),
    )
    .await;
}

#[rstest]
#[tokio::test]
async fn test_swap_many(#[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips) {
    let env = Env::builder().fee(fee).build().await;

    let ft1_token_id = TokenId::Nep141(env.ft1.clone());
    let ft2_token_id = TokenId::Nep141(env.ft2.clone());
    let ft3_token_id = TokenId::Nep141(env.ft3.clone());

    test_ft_diffs(
        &env,
        [
            AccountFtDiff {
                account: &env.user1,
                init_balances: [(&env.ft1, 100)].into_iter().collect(),
                diff: [TokenDeltas::default()
                    .with_add_deltas([(ft1_token_id.clone(), -100), (ft2_token_id.clone(), 200)])
                    .unwrap()]
                .into(),
                result_balances: [(&env.ft2, 200)].into_iter().collect(),
            },
            AccountFtDiff {
                account: &env.user2,
                init_balances: [(&env.ft2, 1000)].into_iter().collect(),
                diff: [
                    TokenDeltas::default()
                        .with_add_deltas([
                            (
                                ft1_token_id.clone(),
                                TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                            ),
                            (
                                ft2_token_id.clone(),
                                TokenDiff::closure_delta(&ft2_token_id, 200, fee).unwrap(),
                            ),
                        ])
                        .unwrap(),
                    TokenDeltas::default()
                        .with_add_deltas([
                            (
                                ft2_token_id.clone(),
                                TokenDiff::closure_delta(&ft2_token_id, 300, fee).unwrap(),
                            ),
                            (
                                ft3_token_id.clone(),
                                TokenDiff::closure_delta(&ft3_token_id, -500, fee).unwrap(),
                            ),
                        ])
                        .unwrap(),
                ]
                .into(),
                result_balances: [
                    (
                        &env.ft1,
                        TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                    ),
                    (
                        &env.ft2,
                        1000 + TokenDiff::closure_delta(&ft2_token_id, 200, fee).unwrap()
                            + TokenDiff::closure_delta(&ft2_token_id, 300, fee).unwrap(),
                    ),
                    (
                        &env.ft3,
                        TokenDiff::closure_delta(&ft3_token_id, -500, fee).unwrap(),
                    ),
                ]
                .into_iter()
                .collect(),
            },
            AccountFtDiff {
                account: &env.user3,
                init_balances: [(&env.ft3, 500)].into_iter().collect(),
                diff: [TokenDeltas::default()
                    .with_add_deltas([(ft2_token_id.clone(), 300), (ft3_token_id.clone(), -500)])
                    .unwrap()]
                .into(),
                result_balances: [(&env.ft2, 300)].into_iter().collect(),
            },
        ]
        .into(),
    )
    .await;
}

type FtBalances<'a> = BTreeMap<&'a AccountId, i128>;

#[derive(Debug)]
struct AccountFtDiff<'a> {
    account: &'a Account,
    init_balances: FtBalances<'a>,
    diff: Vec<TokenDeltas>,
    result_balances: FtBalances<'a>,
}

async fn test_ft_diffs(env: &Env, accounts: Vec<AccountFtDiff<'_>>) {
    // deposit
    for account in &accounts {
        for (token_id, balance) in &account.init_balances {
            env.defuse_ft_mint(
                token_id,
                (*balance).try_into().unwrap(),
                account.account.id(),
            )
            .await
            .unwrap();
        }
    }

    let signed: Vec<MultiPayload> = accounts
        .iter()
        .flat_map(move |account| {
            account.diff.iter().cloned().map(|diff| {
                account.account.sign_defuse_message(
                    env.defuse.id(),
                    make_true_rng().random(),
                    Deadline::timeout(Duration::from_secs(120)),
                    DefuseIntents {
                        intents: [TokenDiff {
                            diff,
                            memo: None,
                            referral: None,
                        }
                        .into()]
                        .into(),
                    },
                )
            })
        })
        .collect();

    // simulate
    env.defuse
        .simulate_intents(signed.clone())
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // verify
    env.defuse.execute_intents(signed).await.unwrap();

    // check balances
    for account in accounts {
        let (tokens, balances): (Vec<_>, Vec<_>) = account
            .result_balances
            .into_iter()
            .map(|(t, b)| {
                (
                    TokenId::Nep141(t.clone()).to_string(),
                    u128::try_from(b).unwrap(),
                )
            })
            .unzip();
        assert_eq!(
            env.mt_contract_batch_balance_of(env.defuse.id(), account.account.id(), &tokens)
                .await
                .unwrap(),
            balances
        );
    }
}

#[tokio::test]
async fn test_invariant_violated() {
    let env = Env::new().await;

    let ft1 = TokenId::Nep141(env.ft1.clone());
    let ft2 = TokenId::Nep141(env.ft2.clone());

    // deposit
    env.defuse_ft_mint(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();
    env.defuse_ft_mint(&env.ft2, 2000, env.user2.id())
        .await
        .unwrap();

    let signed: Vec<_> = [
        env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::MAX,
            DefuseIntents {
                intents: [TokenDiff {
                    diff: TokenDeltas::default()
                        .with_add_deltas([(ft1.clone(), -1000), (ft2.clone(), 2000)])
                        .unwrap(),
                    memo: None,
                    referral: None,
                }
                .into()]
                .into(),
            },
        ),
        env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::MAX,
            DefuseIntents {
                intents: [TokenDiff {
                    diff: TokenDeltas::default()
                        .with_add_deltas([(ft1.clone(), 1000), (ft2.clone(), -1999)])
                        .unwrap(),
                    memo: None,
                    referral: None,
                }
                .into()]
                .into(),
            },
        ),
    ]
    .into();

    assert_eq!(
        env.defuse
            .simulate_intents(signed.clone())
            .await
            .unwrap()
            .invariant_violated
            .unwrap()
            .into_unmatched_deltas(),
        Some(TokenDeltas::new([(ft2.clone(), 1)].into_iter().collect()))
    );

    env.defuse.execute_intents(signed).await.unwrap_err();

    // balances should stay the same
    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            env.user1.id(),
            [&ft1.to_string(), &ft2.to_string()]
        )
        .await
        .unwrap(),
        [1000, 0]
    );
    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            env.user2.id(),
            [&ft1.to_string(), &ft2.to_string()]
        )
        .await
        .unwrap(),
        [0, 2000]
    );
}

#[rstest]
#[tokio::test]
async fn test_solver_user_closure(
    #[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips,
) {
    const USER_BALANCE: u128 = 1100;
    const SOLVER_BALANCE: u128 = 2100;

    // RFQ: 1000 token_in -> ??? token_out
    const USER_DELTA_IN: i128 = -1000;

    let env = Env::builder().fee(fee).build().await;

    let user = &env.user1;
    let solver = &env.user2;

    // deposit
    env.defuse_ft_mint(&env.ft1, USER_BALANCE, user.id())
        .await
        .unwrap();
    env.defuse_ft_mint(&env.ft2, SOLVER_BALANCE, solver.id())
        .await
        .unwrap();

    let token_in = TokenId::Nep141(env.ft1.clone());
    let token_out = TokenId::Nep141(env.ft2.clone());

    dbg!(USER_DELTA_IN);
    // propagate RFQ to solver with adjusted amount_in
    let solver_delta_in = TokenDiff::closure_delta(&token_in, USER_DELTA_IN, fee).unwrap();

    // assume solver trades 1:2
    let solver_delta_out = solver_delta_in * -2;
    dbg!(solver_delta_in, solver_delta_out);

    // solver signs his intent
    let solver_commitment = solver.sign_defuse_message(
        env.defuse.id(),
        make_true_rng().random(),
        Deadline::timeout(Duration::from_secs(90)),
        DefuseIntents {
            intents: [TokenDiff {
                diff: TokenDeltas::new(
                    [
                        (token_in.clone(), solver_delta_in),
                        (token_out.clone(), solver_delta_out),
                    ]
                    .into_iter()
                    .collect(),
                ),
                memo: None,
                referral: None,
            }
            .into()]
            .into(),
        },
    );

    // simulate before returning quote
    let simulation_before_return_quote = env
        .defuse
        .simulate_intents([solver_commitment.clone()])
        .await
        .unwrap();
    println!(
        "simulation_before_return_quote: {}",
        serde_json::to_string_pretty(&simulation_before_return_quote).unwrap()
    );

    // we expect unmatched deltas to correspond with user_delta_in and
    // user_delta_out and fee
    let unmatched_deltas = simulation_before_return_quote
        .invariant_violated
        .unwrap()
        .into_unmatched_deltas()
        .unwrap();
    // there should be unmatched deltas only for 2 tokens: token_in and token_out
    assert_eq!(unmatched_deltas.len(), 2);

    // expect unmatched delta on token_in to be fully covered by user_in
    let expected_unmatched_delta_token_in =
        TokenDiff::closure_delta(&token_in, USER_DELTA_IN, fee).unwrap();
    assert_eq!(
        unmatched_deltas.balance_of(&token_in),
        expected_unmatched_delta_token_in
    );

    // calculate user_delta_out to return to the user
    let user_delta_out =
        TokenDiff::closure_supply_delta(&token_out, unmatched_deltas.balance_of(&token_out), fee)
            .unwrap();
    dbg!(user_delta_out);

    // user signs the message
    let user_commitment = user.sign_defuse_message(
        env.defuse.id(),
        make_true_rng().random(),
        Deadline::timeout(Duration::from_secs(90)),
        DefuseIntents {
            intents: [TokenDiff {
                diff: TokenDeltas::new(
                    [
                        (token_in.clone(), USER_DELTA_IN),
                        (token_out.clone(), user_delta_out),
                    ]
                    .into_iter()
                    .collect(),
                ),
                memo: None,
                referral: None,
            }
            .into()]
            .into(),
        },
    );

    // simulating both solver's and user's intents now should succeed
    env.defuse
        .simulate_intents([solver_commitment.clone(), user_commitment.clone()])
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // execute intents
    env.defuse
        .execute_intents([solver_commitment, user_commitment])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            user.id(),
            [&token_in.to_string(), &token_out.to_string()]
        )
        .await
        .unwrap(),
        [
            USER_BALANCE - USER_DELTA_IN.unsigned_abs(),
            user_delta_out.unsigned_abs()
        ]
    );

    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            solver.id(),
            [&token_in.to_string(), &token_out.to_string()]
        )
        .await
        .unwrap(),
        [
            solver_delta_in.unsigned_abs(),
            SOLVER_BALANCE - solver_delta_out.unsigned_abs()
        ]
    );
}
