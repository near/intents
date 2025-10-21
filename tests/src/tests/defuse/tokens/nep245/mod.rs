mod letter_gen;
mod mt_transfer_resolve_gas;
pub mod traits;

use crate::tests::defuse::DefuseExt;
use crate::tests::defuse::env::PersistentState;
use crate::tests::defuse::tokens::nep245::traits::DefuseMtWithdrawer;
use crate::{tests::defuse::env::Env, utils::mt::MtExt};
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::fees::{FeesConfig, Pips};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::nep245::Token;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn multitoken_enumeration(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;

    let env = Env::builder()
        .no_registration(no_registration)
        .create_unique_users()
        .build()
        .await;

    let (user1, user2, user3, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1, &ft2])
        .await;

    // Check already existing tokens from persistent state
    let persistent_tokens: Vec<Token> = env
        .persistent_state
        .as_ref()
        .map(PersistentState::get_mt_tokens)
        .unwrap_or_default();

    let from_token_index = persistent_tokens.len();

    {
        assert_eq!(
            user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            persistent_tokens
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

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id())
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.clone()));

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

    env.defuse_ft_deposit_to(&ft1, 2000, user2.id())
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

    env.defuse_ft_deposit_to(&ft2, 5000, user1.id())
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
            persistent_tokens
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
async fn multitoken_enumeration_with_ranges(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    let env = Env::builder()
        .no_registration(no_registration)
        .create_unique_users()
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

    env.initial_ft_storage_deposit(vec![user1.id()], vec![&ft1, &ft2, &ft3])
        .await;

    // Check already existing tokens from persistent state
    let persistent_tokens: Vec<Token> = env
        .persistent_state
        .as_ref()
        .map(PersistentState::get_mt_tokens)
        .unwrap_or_default();

    {
        assert_eq!(
            user1.mt_tokens(env.defuse.id(), ..).await.unwrap(),
            persistent_tokens
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

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id())
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft2, 2000, user1.id())
        .await
        .unwrap();
    env.defuse_ft_deposit_to(&ft3, 3000, user1.id())
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

        let from_token = persistent_tokens.len();

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

    // Check already existing tokens from persistent state
    let persistent_tokens: Vec<Token> = env
        .persistent_state
        .as_ref()
        .map(PersistentState::get_mt_tokens)
        .unwrap_or_default();

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
            persistent_tokens
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

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft2, 5000, user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft3, 8000, user1.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft1, 1000, user2.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft2, 5000, user2.id())
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft3, 8000, user2.id())
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3_id.to_string()).unwrap()
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3_id.to_string()).unwrap()
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft1_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft2_id.to_string()).unwrap()
                    )
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
                    &TokenId::Nep245(
                        Nep245TokenId::new(env.defuse.id().to_owned(), ft3_id.to_string()).unwrap()
                    )
                    .to_string(),
                )
                .await
                .unwrap(),
            270
        );
    }
}
