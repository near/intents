use crate::{
    tests::defuse::{DefuseSignerExt, env::Env},
    utils::mt::MtExt,
};
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::core::{
    fees::Pips,
    intents::token_diff::{TokenDeltas, TokenDiff},
};
use near_sdk::AccountId;
use near_workspaces::Account;
use rstest::rstest;
use std::collections::BTreeMap;

use super::ExecuteIntentsExt;

#[rstest]
#[tokio::test]
#[trace]
async fn swap_p2p(
    #[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips,
    #[values(false, true)] no_registration: bool,
) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    let env = Env::builder()
        .fee(fee)
        .no_registration(no_registration)
        .build()
        .await;

    let (user1, user2, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1, &ft2])
        .await;

    test_ft_diffs(
        &env,
        [
            AccountFtDiff {
                account: &user1,
                init_balances: std::iter::once((&ft1, 100)).collect(),
                diff: [TokenDeltas::default()
                    .with_apply_deltas([
                        (ft1_token_id.clone(), -100),
                        (
                            ft2_token_id.clone(),
                            TokenDiff::closure_delta(&ft2_token_id, -200, fee).unwrap(),
                        ),
                    ])
                    .unwrap()]
                .into(),
                result_balances: std::iter::once((
                    &ft2,
                    TokenDiff::closure_delta(&ft2_token_id, -200, fee).unwrap(),
                ))
                .collect(),
            },
            AccountFtDiff {
                account: &user2,
                init_balances: std::iter::once((&ft2, 200)).collect(),
                diff: [TokenDeltas::default()
                    .with_apply_deltas([
                        (
                            ft1_token_id.clone(),
                            TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                        ),
                        (ft2_token_id.clone(), -200),
                    ])
                    .unwrap()]
                .into(),
                result_balances: std::iter::once((
                    &ft1,
                    TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                ))
                .collect(),
            },
        ]
        .into(),
    )
    .await;
}

#[rstest]
#[tokio::test]
#[trace]
async fn swap_many(
    #[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips,
    #[values(false, true)] no_registration: bool,
) {
    let env = Env::builder()
        .fee(fee)
        .no_registration(no_registration)
        .build()
        .await;

    let (user1, user2, user3, ft1, ft2, ft3) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token(),
        env.create_token()
    );

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(ft2.clone()));
    let ft3_token_id = TokenId::from(Nep141TokenId::new(ft3.clone()));

    env.initial_ft_storage_deposit(
        vec![user1.id(), user2.id(), user3.id()],
        vec![&ft1, &ft2, &ft3],
    )
    .await;

    test_ft_diffs(
        &env,
        [
            AccountFtDiff {
                account: &user1,
                init_balances: std::iter::once((&ft1, 100)).collect(),
                diff: [TokenDeltas::default()
                    .with_apply_deltas([(ft1_token_id.clone(), -100), (ft2_token_id.clone(), 200)])
                    .unwrap()]
                .into(),
                result_balances: std::iter::once((&ft2, 200)).collect(),
            },
            AccountFtDiff {
                account: &user2,
                init_balances: std::iter::once((&ft2, 1000)).collect(),
                diff: [
                    TokenDeltas::default()
                        .with_apply_deltas([
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
                        .with_apply_deltas([
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
                        &ft1,
                        TokenDiff::closure_delta(&ft1_token_id, -100, fee).unwrap(),
                    ),
                    (
                        &ft2,
                        1000 + TokenDiff::closure_delta(&ft2_token_id, 200, fee).unwrap()
                            + TokenDiff::closure_delta(&ft2_token_id, 300, fee).unwrap(),
                    ),
                    (
                        &ft3,
                        TokenDiff::closure_delta(&ft3_token_id, -500, fee).unwrap(),
                    ),
                ]
                .into_iter()
                .collect(),
            },
            AccountFtDiff {
                account: &user3,
                init_balances: std::iter::once((&ft3, 500)).collect(),
                diff: [TokenDeltas::default()
                    .with_apply_deltas([(ft2_token_id.clone(), 300), (ft3_token_id.clone(), -500)])
                    .unwrap()]
                .into(),
                result_balances: std::iter::once((&ft2, 300)).collect(),
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
    futures::future::try_join_all(accounts.iter().flat_map(move |account| {
        account
            .init_balances
            .iter()
            .map(move |(token_id, balance)| {
                env.defuse_ft_deposit_to(
                    token_id,
                    (*balance).try_into().unwrap(),
                    account.account.id(),
                    None,
                )
            })
    }))
    .await
    .unwrap();

    let signed = futures::future::try_join_all(accounts.iter().flat_map(move |account| {
        account.diff.iter().cloned().map(move |diff| {
            account.account.sign_defuse_payload_default(
                env.defuse.id(),
                [TokenDiff {
                    diff,
                    memo: None,
                    referral: None,
                }],
            )
        })
    }))
    .await
    .unwrap();

    // simulate
    env.defuse
        .simulate_intents(signed.clone())
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // verify
    env.defuse
        .execute_intents(env.defuse.id(), signed)
        .await
        .unwrap();

    // check balances
    for account in accounts {
        let (tokens, balances): (Vec<_>, Vec<_>) = account
            .result_balances
            .into_iter()
            .map(|(t, b)| {
                (
                    TokenId::from(Nep141TokenId::new(t.clone())).to_string(),
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
#[rstest]
#[trace]
async fn invariant_violated(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user1, user2, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token(),
    );

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1, &ft2])
        .await;

    // deposit
    futures::try_join!(
        env.defuse_ft_deposit_to(&ft1, 1000, user1.id(), None),
        env.defuse_ft_deposit_to(&ft2, 2000, user2.id(), None)
    )
    .expect("Failed to deposit tokens");

    let signed = futures::future::try_join_all([
        user1.sign_defuse_payload_default(
            env.defuse.id(),
            [TokenDiff {
                diff: TokenDeltas::default()
                    .with_apply_deltas([
                        (ft1_token_id.clone(), -1000),
                        (ft2_token_id.clone(), 2000),
                    ])
                    .unwrap(),
                memo: None,
                referral: None,
            }],
        ),
        user1.sign_defuse_payload_default(
            env.defuse.id(),
            [TokenDiff {
                diff: TokenDeltas::default()
                    .with_apply_deltas([
                        (ft1_token_id.clone(), 1000),
                        (ft2_token_id.clone(), -1999),
                    ])
                    .unwrap(),
                memo: None,
                referral: None,
            }],
        ),
    ])
    .await
    .unwrap();

    assert_eq!(
        env.defuse
            .simulate_intents(signed.clone())
            .await
            .unwrap()
            .invariant_violated
            .unwrap()
            .into_unmatched_deltas(),
        Some(TokenDeltas::new(
            std::iter::once((ft2_token_id.clone(), 1)).collect()
        ))
    );

    env.defuse
        .execute_intents(env.defuse.id(), signed)
        .await
        .unwrap_err();

    // balances should stay the same
    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            user1.id(),
            [&ft1_token_id.to_string(), &ft2_token_id.to_string()]
        )
        .await
        .unwrap(),
        [1000, 0]
    );
    assert_eq!(
        env.mt_contract_batch_balance_of(
            env.defuse.id(),
            user2.id(),
            [&ft1_token_id.to_string(), &ft2_token_id.to_string()]
        )
        .await
        .unwrap(),
        [0, 2000]
    );
}

#[rstest]
#[tokio::test]
#[trace]
async fn solver_user_closure(
    #[values(Pips::ZERO, Pips::ONE_BIP, Pips::ONE_PERCENT)] fee: Pips,
    #[values(false, true)] no_registration: bool,
) {
    const USER_BALANCE: u128 = 1100;
    const SOLVER_BALANCE: u128 = 2100;

    // RFQ: 1000 token_in -> ??? token_out
    const USER_DELTA_IN: i128 = -1000;

    let env = Env::builder()
        .fee(fee)
        .no_registration(no_registration)
        .build()
        .await;

    let (user, solver, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user.id(), solver.id()], vec![&ft1, &ft2])
        .await;

    // deposit
    futures::try_join!(
        env.defuse_ft_deposit_to(&ft1, USER_BALANCE, user.id(), None),
        env.defuse_ft_deposit_to(&ft2, SOLVER_BALANCE, solver.id(), None)
    )
    .expect("Failed to deposit tokens");

    let token_in = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let token_out = TokenId::from(Nep141TokenId::new(ft2.clone()));

    dbg!(USER_DELTA_IN);
    // propagate RFQ to solver with adjusted amount_in
    let solver_delta_in = TokenDiff::closure_delta(&token_in, USER_DELTA_IN, fee).unwrap();

    // assume solver trades 1:2
    let solver_delta_out = solver_delta_in * -2;
    dbg!(solver_delta_in, solver_delta_out);

    // solver signs his intent
    let solver_commitment = solver
        .sign_defuse_payload_default(
            env.defuse.id(),
            [TokenDiff {
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
            }],
        )
        .await
        .unwrap();

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
        unmatched_deltas.amount_for(&token_in),
        expected_unmatched_delta_token_in
    );

    // calculate user_delta_out to return to the user
    let user_delta_out =
        TokenDiff::closure_supply_delta(&token_out, unmatched_deltas.amount_for(&token_out), fee)
            .unwrap();
    dbg!(user_delta_out);

    // user signs the message
    let user_commitment = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [TokenDiff {
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
            }],
        )
        .await
        .unwrap();

    // simulating both solver's and user's intents now should succeed
    env.defuse
        .simulate_intents([solver_commitment.clone(), user_commitment.clone()])
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // execute intents
    env.defuse
        .execute_intents(env.defuse.id(), [solver_commitment, user_commitment])
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
