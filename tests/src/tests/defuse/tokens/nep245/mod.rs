mod letter_gen;
mod mt_transfer_resolve_gas;
pub mod traits;

use crate::{
    assert_a_contains_b,
    tests::defuse::{
        DefuseExt, DefuseSignerExt,
        accounts::AccountManagerExt,
        env::{Env, MT_RECEIVER_STUB_WASM, get_account_public_key},
        tokens::nep245::traits::DefuseMtWithdrawer,
    },
    utils::mt::MtExt,
};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::{
        amounts::Amounts,
        fees::{FeesConfig, Pips},
        intents::tokens::{NotifyOnTransfer, Transfer},
        token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId},
    },
    nep245::{MtBurnEvent, MtEvent, MtTransferEvent, Token},
    tokens::{DepositAction, DepositMessage, ExecuteIntents},
};
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_sdk::{AsNep297Event, json_types::U128};
use rstest::rstest;
use std::borrow::Cow;

#[tokio::test]
#[rstest]
async fn multitoken_enumeration() {
    use defuse::core::token_id::nep141::Nep141TokenId;

    use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;

    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1, &ft2])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

    {
        assert_eq!(
            user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            existing_tokens
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

    let from_token_index = existing_tokens.len();

    {
        assert_eq!(
            user1
                .mt_tokens(env.defuse.id(), from_token_index..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&ft1, 2000, user2.id(), None)
        .await
        .unwrap();

    {
        assert_eq!(
            user1
                .mt_tokens(env.defuse.id(), from_token_index..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&ft2, 5000, user1.id(), None)
        .await
        .unwrap();

    {
        assert_eq!(
            user1
                .mt_tokens(env.defuse.id(), from_token_index..)
                .await
                .unwrap(),
            [
                Token {
                    token_id: ft1_id.to_string(),
                    owner_id: None
                },
                Token {
                    token_id: ft2_id.to_string(),
                    owner_id: None
                }
            ]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            [
                Token {
                    token_id: ft1_id.to_string(),
                    owner_id: None
                },
                Token {
                    token_id: ft2_id.to_string(),
                    owner_id: None
                }
            ]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Going back to zero available balance won't make it appear in mt_tokens
    assert_eq!(
        user1
            .defuse_ft_withdraw(env.defuse.id(), &ft1, user1.id(), 1000, None, None)
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        user2
            .defuse_ft_withdraw(env.defuse.id(), &ft1, user2.id(), 2000, None, None)
            .await
            .unwrap(),
        2000
    );

    {
        assert_eq!(
            user1
                .mt_tokens(env.defuse.id(), from_token_index..)
                .await
                .unwrap(),
            [Token {
                token_id: ft2_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft2_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap(),
            []
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Withdraw back everything left for user1, and we're back to the initial state
    assert_eq!(
        user1
            .defuse_ft_withdraw(env.defuse.id(), &ft2, user1.id(), 5000, None, None)
            .await
            .unwrap(),
        5000
    );

    {
        assert_eq!(
            user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            existing_tokens
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }
}

#[tokio::test]
#[rstest]
async fn multitoken_enumeration_with_ranges() {
    use defuse::core::token_id::nep141::Nep141TokenId;

    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3, ft1, ft2, ft3) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id()], vec![&ft1, &ft2, &ft3])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

    {
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft2, 2000, user1.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft3, 3000, user1.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.clone()));
    let ft3_id = TokenId::from(Nep141TokenId::new(ft3.clone()));

    {
        let expected = [
            Token {
                token_id: ft1_id.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft2_id.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft3_id.to_string(),
                owner_id: None,
            },
        ];

        let from_token = existing_tokens.len();

        assert_eq!(
            user1
                .mt_tokens(env.defuse.id(), from_token..)
                .await
                .unwrap(),
            expected[..]
        );

        for i in 0..=expected.len() {
            assert_eq!(
                user1
                    .mt_tokens(env.defuse.id(), from_token + i..)
                    .await
                    .unwrap(),
                expected[i..]
            );
        }

        for start in 0..expected.len() - 1 {
            for end in start..=expected.len() {
                assert_eq!(
                    user1
                        .mt_tokens(env.defuse.id(), from_token + start..from_token + end)
                        .await
                        .unwrap(),
                    expected[start..end]
                );
            }
        }
    }

    {
        let expected = [
            Token {
                token_id: ft1_id.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft2_id.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft3_id.to_string(),
                owner_id: None,
            },
        ];

        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        assert_eq!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        for i in 0..=3 {
            assert_eq!(
                user1
                    .mt_tokens_for_owner(env.defuse.id(), user1.id(), i..)
                    .await
                    .unwrap(),
                expected[i..]
            );
        }

        for i in 0..=3 {
            assert_eq!(
                user1
                    .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..i)
                    .await
                    .unwrap(),
                expected[..i]
            );
        }

        for i in 1..=3 {
            assert_eq!(
                user1
                    .mt_tokens_for_owner(env.defuse.id(), user1.id(), 1..i)
                    .await
                    .unwrap(),
                expected[1..i]
            );
        }

        for i in 2..=3 {
            assert_eq!(
                user1
                    .mt_tokens_for_owner(env.defuse.id(), user1.id(), 2..i)
                    .await
                    .unwrap(),
                expected[2..i]
            );
        }
    }
}

#[tokio::test]
#[rstest]
async fn multitoken_withdrawals() {
    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3, ft1, ft2, ft3) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id()], vec![&ft1, &ft2, &ft3])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    {
        assert_eq!(
            user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            existing_tokens
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            user1
                .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft2, 5000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft3, 8000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft1, 1000, user2.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft2, 5000, user2.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft3, 8000, user2.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::Nep141(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::Nep141(Nep141TokenId::new(ft2.clone()));
    let ft3_id = TokenId::Nep141(Nep141TokenId::new(ft3.clone()));

    // At this point, user1 in defuse2, has no balance of `"nep245:defuse.test.near:nep141:ft1.test.near"`, and others. We will fund it next.
    {
        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft1_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft2_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft3_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );
    }

    // Do an mt_transfer_call, and the message is of type DepositMessage, which will contain the user who will own the tokens in defuse2
    {
        user1
            .mt_transfer_call(
                env.defuse.id(),
                defuse2.id(),
                &ft1_id.to_string(),
                100,
                None,
                None,
                user1.id().to_string(),
            )
            .await
            .unwrap();

        user1
            .mt_transfer_call(
                env.defuse.id(),
                defuse2.id(),
                &ft2_id.to_string(),
                200,
                None,
                None,
                user1.id().to_string(),
            )
            .await
            .unwrap();

        user1
            .mt_transfer_call(
                env.defuse.id(),
                defuse2.id(),
                &ft3_id.to_string(),
                300,
                None,
                None,
                user1.id().to_string(),
            )
            .await
            .unwrap();
    }

    // At this point, user1 in defuse2 has 100 of `"nep245:defuse.test.near:nep141:ft1.test.near"`, and others
    {
        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft1_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            100
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft2_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            200
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft3_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            300
        );
    }

    // To use this:
    // 1. Add this to the resolve function: near_sdk::log!("225a33ac58aee0cf8c6ea225223a237a");
    // 2. Uncomment the key string, which is used to detect this log in promises
    // 3. Uncomment the code after mt_withdraw calls, and it will print the gas values
    // let key_string = "225a33ac58aee0cf8c6ea225223a237a";

    // Now we do a withdraw of ft1 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![(ft1_id.to_string(), 10)];

        let (_amounts, _test_log) = user1
            .defuse_mt_withdraw(
                defuse2.id(),
                env.defuse.id(),
                user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
                None,
            )
            .await
            .unwrap();

        // let receipt_logs = test_log
        //     .logs_and_gas_burnt_in_receipts()
        //     .iter()
        //     .filter(|(a, _b)| a.iter().any(|s| s.contains(key_string)))
        //     .collect::<Vec<_>>();
        // assert_eq!(receipt_logs.len(), 1);

        // println!("Cost with token count 1: {}", receipt_logs[0].1);
    }

    // Now user1 in defuse2 has 90 tokens left of `"nep245:defuse.test.near:nep141:ft1.test.near"`
    {
        // Only ft1 balance changes
        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft1_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            90
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft2_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            200
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft3_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            300
        );
    }

    // Now we do a withdraw of ft1 and ft2 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![(ft1_id.to_string(), 10), (ft2_id.to_string(), 20)];

        let (_amounts, _test_log) = user1
            .defuse_mt_withdraw(
                defuse2.id(),
                env.defuse.id(),
                user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
                None,
            )
            .await
            .unwrap();

        // let receipt_logs = test_log
        //     .logs_and_gas_burnt_in_receipts()
        //     .iter()
        //     .filter(|(a, _b)| a.iter().any(|s| s.contains(key_string)))
        //     .collect::<Vec<_>>();
        // assert_eq!(receipt_logs.len(), 1);

        // println!("Cost with token count 2: {}", receipt_logs[0].1);
    }

    // Now we do a withdraw of ft1, ft2 and ft3 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![
            (ft1_id.to_string(), 10),
            (ft2_id.to_string(), 20),
            (ft3_id.to_string(), 30),
        ];

        let (_amounts, _test_log) = user1
            .defuse_mt_withdraw(
                defuse2.id(),
                env.defuse.id(),
                user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
                None,
            )
            .await
            .unwrap();

        // let receipt_logs = test_log
        //     .logs_and_gas_burnt_in_receipts()
        //     .iter()
        //     .filter(|(a, _b)| a.iter().any(|s| s.contains(key_string)))
        //     .collect::<Vec<_>>();
        // assert_eq!(receipt_logs.len(), 1);

        // println!("Cost with token count 3: {}", receipt_logs[0].1);
    }

    // We ensure the math is sound after the last two withdrawals
    {
        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft1_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            70
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft2_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            160
        );

        assert_eq!(
            defuse2
                .mt_balance_of(
                    user1.id(),
                    &TokenId::Nep245(Nep245TokenId::new(
                        env.defuse.id().to_owned(),
                        ft3_id.to_string()
                    ))
                    .to_string(),
                )
                .await
                .unwrap(),
            270
        );
    }
}

#[derive(Debug, Clone)]
struct MtTransferCallExpectation {
    action: StubAction,
    intent_transfer_amounts: Option<Vec<u128>>,
    refund_if_fails: bool,
    expected_sender_mt_balances: Vec<u128>,
    expected_receiver_mt_balances: Vec<u128>,
}

#[tokio::test]
#[rstest]
#[case::receiver_accepts_all_tokens_no_refund(MtTransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![0],
    expected_receiver_mt_balances: vec![1000],
})]
#[case::receiver_requests_partial_refund_300_of_1000(MtTransferCallExpectation {
    action: StubAction::ReturnValue(300.into()),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![300],
    expected_receiver_mt_balances: vec![700],
})]
#[case::receiver_requests_excessive_refund_capped_at_transferred_amount(MtTransferCallExpectation {
    action: StubAction::ReturnValue(2_000.into()),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1_000],
    expected_receiver_mt_balances: vec![0],
})]
#[case::receiver_panics_no_refund_sender_loses_tokens(MtTransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000],
    expected_receiver_mt_balances: vec![0],
})]
#[case::receiver_returns_oversized_data_no_refund_sender_loses_tokens(MtTransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000],
    expected_receiver_mt_balances: vec![0],
})]
async fn mt_transfer_call_calls_mt_on_transfer_single_token(
    #[case] expectation: MtTransferCallExpectation,
) {
    use crate::tests::defuse::DefuseSignerExt;
    use crate::tests::defuse::env::MT_RECEIVER_STUB_WASM;
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};

    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, intent_receiver, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    // Deploy second defuse instance as the receiver
    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    // Deploy stub receiver for testing mt_on_transfer behavior
    let receiver = env.create_user().await;
    receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    // Register receiver's public key in defuse2 so it can execute intents
    receiver
        .add_public_key(defuse2.id(), get_account_public_key(&receiver))
        .await
        .unwrap();

    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![&ft],
    )
    .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    // Fund user with tokens in defuse1
    env.defuse_ft_deposit_to(&ft, 1000, user.id(), None)
        .await
        .unwrap();

    // Get the nep245 token id for defuse1's wrapped token in defuse2
    let nep245_ft_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft_id.to_string(),
    ));

    // Build transfer intent if specified
    let intents = match &expectation.intent_transfer_amounts {
        Some(amounts) if !amounts.is_empty() => {
            vec![
                receiver
                    .sign_defuse_payload_default(
                        defuse2.id(),
                        [Transfer {
                            receiver_id: intent_receiver.id().clone(),
                            tokens: Amounts::new(
                                std::iter::once((nep245_ft_id.clone(), amounts[0])).collect(),
                            ),
                            memo: None,
                            notification: None,
                        }],
                    )
                    .await
                    .unwrap(),
            ]
        }
        _ => vec![],
    };

    let deposit_message = if intents.is_empty() {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    // Transfer from defuse1 to defuse2 using mt_transfer_call
    user.mt_transfer_call(
        env.defuse.id(),
        defuse2.id(),
        &ft_id.to_string(),
        1000,
        None,
        None,
        near_sdk::serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    // Check balances in defuse1 (original sender)
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[0],
        "Sender balance in defuse1 should match expected"
    );

    // Check balances in defuse2 (receiver) - token is wrapped as NEP-245
    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), receiver.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[0],
        "Receiver balance in defuse2 should match expected"
    );
}

#[tokio::test]
#[rstest]
#[case::nothing_to_refund_multi_token(MtTransferCallExpectation {
    action: StubAction::ReturnValues(vec![0.into(), 0.into()]),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![0, 0],
    expected_receiver_mt_balances: vec![1000, 2000],
})]
#[case::partial_refund_first_token(MtTransferCallExpectation {
    action: StubAction::ReturnValues(vec![300.into(), 0.into()]),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![300, 0],
    expected_receiver_mt_balances: vec![700, 2000],
})]
#[case::malicious_refund_multi_token(MtTransferCallExpectation {
    action: StubAction::ReturnValues(vec![3_000.into(), 3_000.into()]),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000, 2000],
    expected_receiver_mt_balances: vec![0, 0],
})]
#[case::receiver_panics_multi_token(MtTransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000, 2000],
    expected_receiver_mt_balances: vec![0, 0],
})]
#[case::malicious_receiver_multi_token(MtTransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000, 2000],
    expected_receiver_mt_balances: vec![0, 0],
})]
#[case::wrong_length_return_too_short(MtTransferCallExpectation {
    action: StubAction::ReturnValues(vec![100.into()]),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000, 2000],
    expected_receiver_mt_balances: vec![0, 0],
})]
#[case::wrong_length_return_too_long(MtTransferCallExpectation {
    action: StubAction::ReturnValues(vec![100.into(), 200.into(), 300.into()]),
    intent_transfer_amounts: None,
    refund_if_fails: true,
    expected_sender_mt_balances: vec![1000, 2000],
    expected_receiver_mt_balances: vec![0, 0],
})]
async fn mt_transfer_call_calls_mt_on_transfer_multi_token(
    #[case] expectation: MtTransferCallExpectation,
) {
    use crate::tests::defuse::DefuseSignerExt;
    use crate::tests::defuse::env::MT_RECEIVER_STUB_WASM;
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};
    use defuse::tokens::DepositMessage;

    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, intent_receiver, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    // Deploy second defuse instance as the receiver
    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    // Deploy stub receiver for testing mt_on_transfer behavior
    let receiver = env.create_user().await;
    receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    // Register receiver's public key in defuse2 so it can execute intents
    receiver
        .add_public_key(defuse2.id(), get_account_public_key(&receiver))
        .await
        .unwrap();

    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![&ft1, &ft2],
    )
    .await;

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

    // Fund user with tokens in defuse1
    env.defuse_ft_deposit_to(&ft1, 1000, user.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft2, 2000, user.id(), None)
        .await
        .unwrap();

    // Get the nep245 token ids for defuse1's wrapped tokens in defuse2
    let nep245_ft1_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft1_id.to_string(),
    ));
    let nep245_ft2_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft2_id.to_string(),
    ));

    // Build transfer intents if specified
    let intents = if let Some(amounts) = &expectation.intent_transfer_amounts {
        let mut intent_map = std::collections::BTreeMap::new();

        if let Some(&amount1) = amounts.first() {
            intent_map.insert(nep245_ft1_id.clone(), amount1);
        }
        if let Some(&amount2) = amounts.get(1) {
            intent_map.insert(nep245_ft2_id.clone(), amount2);
        }

        vec![
            receiver
                .sign_defuse_payload_default(
                    defuse2.id(),
                    [Transfer {
                        receiver_id: intent_receiver.id().clone(),
                        tokens: Amounts::new(intent_map),
                        memo: None,
                        notification: None,
                    }],
                )
                .await
                .unwrap(),
        ]
    } else {
        vec![]
    };

    let deposit_message = if intents.is_empty() {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    // Transfer both tokens from user in defuse1 to defuse2 using batch transfer
    let _ = user
        .call(env.defuse.id(), "mt_batch_transfer_call")
        .deposit(near_workspaces::types::NearToken::from_yoctonear(1))
        .args_json(near_sdk::serde_json::json!({
            "receiver_id": defuse2.id(),
            "token_ids": vec![ft1_id.to_string(), ft2_id.to_string()],
            "amounts": vec![near_sdk::json_types::U128(1000), near_sdk::json_types::U128(2000)],
            "approvals": Option::<Vec<Option<(near_sdk::AccountId, u64)>>>::None,
            "memo": Option::<String>::None,
            "msg": near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // Check balances in defuse1 (original sender)
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[0],
        "Sender balance for ft1 in defuse1 should match expected"
    );
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft2_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[1],
        "Sender balance for ft2 in defuse1 should match expected"
    );

    // Check balances in defuse2 (receiver)
    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), receiver.id(), &nep245_ft1_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[0],
        "Receiver balance for ft1 in defuse2 should match expected"
    );
    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), receiver.id(), &nep245_ft2_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[1],
        "Receiver balance for ft2 in defuse2 should match expected"
    );
}

#[tokio::test]
async fn mt_transfer_call_circullar_callback() {
    use defuse::tokens::DepositMessage;

    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    // Step 1: Deposit tokens to user in defuse1
    env.defuse_ft_deposit_to(&ft, 1000, user.id(), None)
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000,
        "User should have 1000 tokens in defuse1"
    );

    // NOTE: Test circular callback case: defuse1 → defuse2 → defuse1
    // Set receiver_id to defuse1 to create circular callback
    // With empty inner message to avoid further callbacks
    let deposit_message = DepositMessage {
        receiver_id: env.defuse.id().clone(), // Circular: back to defuse1
        action: Some(DepositAction::Notify(NotifyOnTransfer::new(
            serde_json::to_string(&DepositMessage::new(user.id().clone())).unwrap(),
        ))),
    };

    // Get the nep245 token id for defuse1's wrapped token in defuse2
    let nep245_ft_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft_id.to_string(),
    ));

    let refund_amounts = user
        .mt_transfer_call(
            env.defuse.id(),
            defuse2.id(),
            &ft_id.to_string(),
            600,
            None,
            None,
            near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        )
        .await
        .expect("mt_transfer_call should succeed");

    // The inner callback to defuse1 should succeed and keep all tokens
    assert_eq!(
        refund_amounts,
        vec![600],
        "Should return 600 (amount used) since tokens were successfully deposited in circular callback"
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        400,
        "User should have 400 tokens in defuse1 after transfer"
    );

    // In the circular callback flow:
    // 1. defuse2 receives 600 tokens, deposits them to defuse1 (receiver_id in outer message)
    // 2. defuse2 calls defuse1.mt_on_transfer as a notification (with inner message)
    // 3. defuse1.mt_on_transfer processes the notification and returns no refund
    //
    // IMPORTANT: mt_on_transfer is just a notification callback, it doesn't transfer tokens again.
    // The tokens are already deposited in defuse2, owned by defuse1.

    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), env.defuse.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        600,
        "defuse1 should have 600 wrapped tokens in defuse2 after circular callback"
    );

    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), user.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        0,
        "User should have 0 wrapped tokens in defuse2"
    );
}

#[tokio::test]
async fn mt_transfer_call_circullar_deposit() {
    use defuse::tokens::DepositMessage;

    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();
    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    // Step 1: Deposit tokens to defuse2 in defuse1
    env.defuse_ft_deposit_to(
        &ft,
        1000,
        defuse2.id(),
        // NOTE: Test circular callback case: defuse2 → defuse1
        // Set receiver_id to defuse1 to create circular callback
        // With empty inner message to avoid further callbacks
        DepositAction::Notify(NotifyOnTransfer::new(
            serde_json::to_string(&DepositMessage {
                receiver_id: env.defuse.id().clone(), // Circular: back to defuse1
                action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                    serde_json::to_string(&DepositMessage::new(user.id().clone())).unwrap(),
                ))),
            })
            .unwrap(),
        )),
    )
    .await
    .unwrap();

    // Get the nep245 token id for defuse1
    let defuse1_ft_id: TokenId = Nep141TokenId::new(ft.clone()).into();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), defuse2.id(), &defuse1_ft_id.to_string())
            .await
            .unwrap(),
        1000,
        "defuse2 should have 1000 tokens in defuse1"
    );

    let defuse2_nep245_ft_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        defuse1_ft_id.to_string(),
    ));

    assert_eq!(
        env.mt_contract_balance_of(
            defuse2.id(),
            env.defuse.id(),
            &defuse2_nep245_ft_id.to_string()
        )
        .await
        .unwrap(),
        1000,
        "defuse1 should have 1000 tokens in defuse2 after wrapping"
    );

    let defuse1_defuse2_nep245_ft_id = TokenId::Nep245(Nep245TokenId::new(
        defuse2.id().clone(),
        defuse2_nep245_ft_id.to_string(),
    ));

    assert_eq!(
        env.mt_contract_balance_of(
            env.defuse.id(),
            user.id(),
            &defuse1_defuse2_nep245_ft_id.to_string()
        )
        .await
        .unwrap(),
        1000,
        "user should have 1000 tokens in defuse1 after wrapping via defuse2"
    );
}

#[tokio::test]
async fn mt_transfer_call_duplicate_tokens_with_stub_execute_and_refund() {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, another_receiver, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    let stub_receiver = env.create_user().await;
    stub_receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    // Register stub's public key in defuse2 so it can execute intents
    stub_receiver
        .add_public_key(defuse2.id(), get_account_public_key(&stub_receiver))
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.id(), stub_receiver.id()], vec![&ft1, &ft2])
        .await;

    let transfer_amounts = [1000, 2000, 3000].map(U128::from).to_vec();
    let refund_amounts = [1000, 2000, 1000].map(U128::from).to_vec();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

    let nep245_ft1_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft1_id.to_string(),
    ));
    let nep245_ft2_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft2_id.to_string(),
    ));

    env.defuse_ft_deposit_to(&ft1, 4000, user.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft2, 2000, user.id(), None)
        .await
        .unwrap();

    let stub_action = StubAction::ExecuteAndRefund {
        multipayload: stub_receiver
            .sign_defuse_payload_default(
                defuse2.id(),
                [Transfer {
                    receiver_id: another_receiver.id().clone(),
                    tokens: Amounts::new([(nep245_ft1_id.clone(), 2000)].into()),
                    memo: None,
                    notification: None,
                }],
            )
            .await
            .unwrap(),
        refund_amounts: refund_amounts.clone(),
    };

    let deposit_message = DepositMessage {
        receiver_id: stub_receiver.id().clone(),
        action: Some(DepositAction::Notify(NotifyOnTransfer::new(
            near_sdk::serde_json::to_string(&stub_action).unwrap(),
        ))),
    };

    let result = user
        .call(env.defuse.id(), "mt_batch_transfer_call")
        .deposit(near_workspaces::types::NearToken::from_yoctonear(1))
        .args_json(near_sdk::serde_json::json!({
            "receiver_id": defuse2.id(),
            "token_ids": vec![ft1_id.to_string(), ft2_id.to_string(), ft1_id.to_string()],
            "amounts": transfer_amounts,
            "approvals": Option::<Vec<Option<(near_sdk::AccountId, u64)>>>::None,
            "memo": Option::<String>::None,
            "msg": near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap();

    println!("\n=== Transaction Logs ===");
    for (i, log) in result.logs().iter().enumerate() {
        println!("[{i}] {log}");
    }

    let all_logs: Vec<String> = result
        .logs()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    let _ = result.into_result().unwrap();

    // Token IDs for events
    let ft_token_ids = [ft1_id.to_string(), ft2_id.to_string(), ft1_id.to_string()];
    let mt_token_ids = [
        nep245_ft1_id.to_string(),
        nep245_ft2_id.to_string(),
        nep245_ft1_id.to_string(),
    ];

    let burn_events = [MtBurnEvent {
        owner_id: Cow::Borrowed(stub_receiver.id().as_ref()),
        authorized_id: None,
        token_ids: Cow::Borrowed(&mt_token_ids),
        amounts: Cow::Borrowed(&refund_amounts),
        memo: Some(Cow::Borrowed("refund")),
    }];
    let expected_mt_burn = MtEvent::MtBurn(Cow::Borrowed(&burn_events));

    let transfer_events = [MtTransferEvent {
        authorized_id: None,
        old_owner_id: Cow::Borrowed(defuse2.id().as_ref()),
        new_owner_id: Cow::Borrowed(user.id().as_ref()),
        token_ids: Cow::Borrowed(&ft_token_ids),
        amounts: Cow::Borrowed(&refund_amounts), // Use capped refund amounts
        memo: Some(Cow::Borrowed("refund")),
    }];
    let expected_mt_transfer = MtEvent::MtTransfer(Cow::Borrowed(&transfer_events));

    assert_a_contains_b!(
        a: all_logs,
        b: [
            expected_mt_burn.to_nep297_event().to_event_log(),
            expected_mt_transfer.to_nep297_event().to_event_log(),
        ]
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1_id.to_string())
            .await
            .unwrap(),
        2000,
        "User should have: 1000 (first refund) + 1000 (third refund capped) = 2000 of token1"
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft2_id.to_string())
            .await
            .unwrap(),
        2000,
        "User should have: 2000 (second refund) = 2000 of token2 (all refunded)"
    );
}
