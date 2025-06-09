pub mod traits;

use crate::tests::defuse::tokens::nep245::traits::DefuseMtWithdrawer;
use crate::{tests::defuse::env::Env, utils::mt::MtExt};
use defuse::contract::gas::total_mt_withdraw_gas;
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::nep245::Token;
use near_sdk::Gas;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn multitoken_enumeration(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    {
        assert!(
            env.user1
                .mt_tokens(env.defuse.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    let ft2 = TokenId::from(Nep141TokenId::new(env.ft2.clone()));

    {
        assert_eq!(
            env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&env.ft1, 2000, env.user2.id())
        .await
        .unwrap();

    {
        assert_eq!(
            env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&env.ft2, 5000, env.user1.id())
        .await
        .unwrap();

    {
        assert_eq!(
            env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            [
                Token {
                    token_id: ft1.to_string(),
                    owner_id: None
                },
                Token {
                    token_id: ft2.to_string(),
                    owner_id: None
                }
            ]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            [
                Token {
                    token_id: ft1.to_string(),
                    owner_id: None
                },
                Token {
                    token_id: ft2.to_string(),
                    owner_id: None
                }
            ]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Going back to zero available balance won't make it appear in mt_tokens
    assert_eq!(
        env.user1
            .defuse_ft_withdraw(env.defuse.id(), &env.ft1, env.user1.id(), 1000)
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.user2
            .defuse_ft_withdraw(env.defuse.id(), &env.ft1, env.user2.id(), 2000)
            .await
            .unwrap(),
        2000
    );

    {
        assert_eq!(
            env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            [Token {
                token_id: ft2.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft2.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap(),
            []
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Withdraw back everything left for user1, and we're back to the initial state
    assert_eq!(
        env.user1
            .defuse_ft_withdraw(env.defuse.id(), &env.ft2, env.user1.id(), 5000)
            .await
            .unwrap(),
        5000
    );

    {
        assert!(
            env.user1
                .mt_tokens(env.defuse.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }
}

#[tokio::test]
#[rstest]
async fn multitoken_enumeration_with_ranges(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    {
        assert!(
            env.user1
                .mt_tokens(env.defuse.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&env.ft2, 2000, env.user1.id())
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&env.ft3, 3000, env.user1.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    let ft2 = TokenId::from(Nep141TokenId::new(env.ft2.clone()));
    let ft3 = TokenId::from(Nep141TokenId::new(env.ft3.clone()));

    {
        let expected = [
            Token {
                token_id: ft1.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft2.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft3.to_string(),
                owner_id: None,
            },
        ];
        assert_eq!(
            env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            expected[..]
        );

        for i in 0..=3 {
            assert_eq!(
                env.user1.mt_tokens(env.defuse.id(), i..).await.unwrap(),
                expected[i..]
            );
        }

        for i in 0..=3 {
            assert_eq!(
                env.user1.mt_tokens(env.defuse.id(), ..i).await.unwrap(),
                expected[..i]
            );
        }

        for i in 1..=3 {
            assert_eq!(
                env.user1.mt_tokens(env.defuse.id(), 1..i).await.unwrap(),
                expected[1..i]
            );
        }

        for i in 2..=3 {
            assert_eq!(
                env.user1.mt_tokens(env.defuse.id(), 2..i).await.unwrap(),
                expected[2..i]
            );
        }
    }

    {
        let expected = [
            Token {
                token_id: ft1.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft2.to_string(),
                owner_id: None,
            },
            Token {
                token_id: ft3.to_string(),
                owner_id: None,
            },
        ];

        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        assert_eq!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        for i in 0..=3 {
            assert_eq!(
                env.user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), i..)
                    .await
                    .unwrap(),
                expected[i..]
            );
        }

        for i in 0..=3 {
            assert_eq!(
                env.user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..i)
                    .await
                    .unwrap(),
                expected[..i]
            );
        }

        for i in 1..=3 {
            assert_eq!(
                env.user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), 1..i)
                    .await
                    .unwrap(),
                expected[1..i]
            );
        }

        for i in 2..=3 {
            assert_eq!(
                env.user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), 2..i)
                    .await
                    .unwrap(),
                expected[2..i]
            );
        }
    }
}

#[tokio::test]
#[rstest]
async fn multitoken_transfers() {
    let env = Env::builder().build().await;

    {
        assert!(
            env.user1
                .mt_tokens(env.defuse.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.user1
                .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft2, 5000, env.user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft3, 8000, env.user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user2.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft2, 5000, env.user2.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft3, 8000, env.user2.id())
        .await
        .unwrap();

    let ft1 = TokenId::Nep141(Nep141TokenId::new(env.ft1.clone()));
    let ft2 = TokenId::Nep141(Nep141TokenId::new(env.ft2.clone()));
    let ft3 = TokenId::Nep141(Nep141TokenId::new(env.ft3.clone()));

    // At this point, user1 in defuse2, has no balance of `"nep245:defuse.test.near:nep141:ft1.test.near"`, and others. We will fund it next.
    {
        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            0
        );
    }

    // Do an mt_transfer_call, and the message is of type DepositMessage, which will contain the user who will own the tokens in defuse2
    {
        env.user1
            .mt_transfer_call(
                env.defuse.id(),
                env.defuse2.id(),
                &ft1.to_string(),
                100,
                None,
                None,
                env.user1.id().to_string(),
            )
            .await
            .unwrap();

        env.user1
            .mt_transfer_call(
                env.defuse.id(),
                env.defuse2.id(),
                &ft2.to_string(),
                200,
                None,
                None,
                env.user1.id().to_string(),
            )
            .await
            .unwrap();

        env.user1
            .mt_transfer_call(
                env.defuse.id(),
                env.defuse2.id(),
                &ft3.to_string(),
                300,
                None,
                None,
                env.user1.id().to_string(),
            )
            .await
            .unwrap();
    }

    // At this point, user1 in defuse2 has 100 of `"nep245:defuse.test.near:nep141:ft1.test.near"`, and others
    {
        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            100
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            200
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            300
        );
    }

    let gas_risk_margin_percent = 30; // 30% - The gas allocated should be 30% more than needed to account for risk

    // Now we do a withdraw of ft1 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![(ft1.to_string(), 10)];

        let (_amounts, test_log) = env
            .user1
            .defuse_mt_withdraw(
                env.defuse2.id(),
                env.defuse.id(),
                env.user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
            )
            .await
            .unwrap();

        // The sum of the receipts is always less than the total gas used, because the main tx is missing
        assert!(
            *test_log.total_gas_burnt()
                > test_log
                    .gas_burnt_in_receipts()
                    .iter()
                    .fold(Gas::from_tgas(0), |so_far, curr| {
                        so_far.checked_add(*curr).unwrap()
                    })
        );

        // Ensure that enough gas will always be allocated
        assert!(
            test_log
                .total_gas_burnt()
                .checked_mul(gas_risk_margin_percent + 100)
                .unwrap()
                .checked_div(100)
                .unwrap()
                < total_mt_withdraw_gas(tokens.len()),
            "The amount of gas spent in the transaction is higher than the expected amount + margin of error. Maybe you should increase the allocated gas."
        );
    }

    // Now user1 in defuse2 has 90 tokens left of `"nep245:defuse.test.near:nep141:ft1.test.near"`
    {
        // Only ft1 balance changes
        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            90
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            200
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            300
        );
    }

    // Now we do a withdraw of ft1 and ft2 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![(ft1.to_string(), 10), (ft2.to_string(), 20)];

        let (_amounts, test_log) = env
            .user1
            .defuse_mt_withdraw(
                env.defuse2.id(),
                env.defuse.id(),
                env.user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
            )
            .await
            .unwrap();

        // The sum of the receipts is always less than the total gas used, because the main tx is missing
        assert!(
            *test_log.total_gas_burnt()
                > test_log
                    .gas_burnt_in_receipts()
                    .iter()
                    .fold(Gas::from_tgas(0), |so_far, curr| {
                        so_far.checked_add(*curr).unwrap()
                    })
        );

        // Ensure that enough gas will always be allocated
        assert!(
            test_log
                .total_gas_burnt()
                .checked_mul(gas_risk_margin_percent + 100)
                .unwrap()
                .checked_div(100)
                .unwrap()
                < total_mt_withdraw_gas(tokens.len()),
            "The amount of gas spent in the transaction is higher than the expected amount + margin of error. Maybe you should increase the allocated gas."
        );
    }

    // Now we do a withdraw of ft1, ft2 and ft3 from defuse2, which will trigger a transfer from defuse2 account in defuse, to user2
    {
        let tokens: Vec<(String, u128)> = vec![
            (ft1.to_string(), 10),
            (ft2.to_string(), 20),
            (ft3.to_string(), 30),
        ];

        let (_amounts, test_log) = env
            .user1
            .defuse_mt_withdraw(
                env.defuse2.id(),
                env.defuse.id(),
                env.user2.id(),
                tokens.iter().cloned().map(|v| v.0).collect(),
                tokens.iter().map(|v| v.1).collect(),
            )
            .await
            .unwrap();

        // The sum of the receipts is always less than the total gas used, because the main tx is missing
        assert!(
            *test_log.total_gas_burnt()
                > test_log
                    .gas_burnt_in_receipts()
                    .iter()
                    .fold(Gas::from_tgas(0), |so_far, curr| {
                        so_far.checked_add(*curr).unwrap()
                    })
        );

        // Ensure that enough gas will always be allocated
        assert!(
            test_log
                .total_gas_burnt()
                .checked_mul(gas_risk_margin_percent + 100)
                .unwrap()
                .checked_div(100)
                .unwrap()
                < total_mt_withdraw_gas(tokens.len()),
            "The amount of gas spent in the transaction is higher than the expected amount + margin of error. Maybe you should increase the allocated gas."
        );
    }

    // We ensure the math is sound after the last two withdrawals
    {
        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            70
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            160
        );

        assert_eq!(
            env.defuse2
                .mt_balance_of(
                    env.user1.id(),
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            270
        );
    }
}
