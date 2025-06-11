use crate::{
    tests::defuse::{env::Env, tokens::nep141::DefuseFtWithdrawer},
    utils::mt::MtExt,
};
use defuse::core::token_id::TokenId;
use defuse::nep245::Token;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn multitoken_enumeration(#[values(false, true)] no_registration: bool) {
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
