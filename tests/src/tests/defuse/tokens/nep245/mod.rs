mod letter_gen;
mod mt_transfer_resolve_gas;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::{Env, MT_RECEIVER_STUB_WASM};
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::amounts::Amounts;
use defuse::core::fees::{FeesConfig, Pips};
use defuse::core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::nep245::Token;
use defuse::nep245::{MtBurnEvent, MtEvent, MtTransferEvent};
use defuse::sandbox_ext::account_manager::AccountManagerExt;
use defuse::sandbox_ext::deployer::DefuseExt;
use defuse::sandbox_ext::tokens::{nep141::DefuseFtWithdrawer, nep245::DefuseMtWithdrawer};
use defuse::tokens::DepositMessage;
use defuse::tokens::nep245::{SelfAction, WithdrawAction};
use defuse::tokens::{DepositAction, ExecuteIntents};
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::mt::{MtExt, MtViewExt};
use defuse_sandbox::tx::FnCallBuilder;
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_sdk::NearToken;
use near_sdk::{AsNep297Event, json_types::U128};
use rstest::rstest;
use std::borrow::Cow;

#[rstest]
#[tokio::test]
async fn multitoken_enumeration() {
    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![ft1.id(), ft2.id()])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = env.defuse.mt_tokens(..).await.unwrap();

    {
        assert_eq!(env.defuse.mt_tokens(..).await.unwrap(), existing_tokens);
        assert!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.id().clone()));

    let from_token_index = existing_tokens.len();

    {
        assert_eq!(
            env.defuse.mt_tokens(from_token_index..).await.unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(ft1.id(), 2000, user2.id(), None)
        .await
        .unwrap();

    {
        assert_eq!(
            env.defuse.mt_tokens(from_token_index..).await.unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(ft2.id(), 5000, user1.id(), None)
        .await
        .unwrap();

    {
        assert_eq!(
            env.defuse.mt_tokens(from_token_index..).await.unwrap(),
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
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
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
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft1_id.to_string(),
                owner_id: None
            }]
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Going back to zero available balance won't make it appear in mt_tokens
    assert_eq!(
        user1
            .defuse_ft_withdraw(env.defuse.id(), ft1.id(), user1.id(), 1000, None, None)
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        user2
            .defuse_ft_withdraw(env.defuse.id(), ft1.id(), user2.id(), 2000, None, None)
            .await
            .unwrap(),
        2000
    );

    {
        assert_eq!(
            env.defuse.mt_tokens(from_token_index..).await.unwrap(),
            [Token {
                token_id: ft2_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap(),
            [Token {
                token_id: ft2_id.to_string(),
                owner_id: None
            }]
        );
        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap(),
            []
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    // Withdraw back everything left for user1, and we're back to the initial state
    assert_eq!(
        user1
            .defuse_ft_withdraw(env.defuse.id(), ft2.id(), user1.id(), 5000, None, None)
            .await
            .unwrap(),
        5000
    );

    {
        assert_eq!(env.defuse.mt_tokens(..).await.unwrap(), existing_tokens);
        assert!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }
}

#[rstest]
#[tokio::test]
async fn multitoken_enumeration_with_ranges() {
    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3, ft1, ft2, ft3) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id()], vec![ft1.id(), ft2.id(), ft3.id()])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = env.defuse.mt_tokens(..).await.unwrap();

    {
        assert!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(ft2.id(), 2000, user1.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(ft3.id(), 3000, user1.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.id().clone()));
    let ft3_id = TokenId::from(Nep141TokenId::new(ft3.id().clone()));

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
            env.defuse.mt_tokens(from_token..).await.unwrap(),
            expected[..]
        );

        for i in 0..=expected.len() {
            assert_eq!(
                env.defuse.mt_tokens(from_token + i..).await.unwrap(),
                expected[i..]
            );
        }

        for start in 0..expected.len() - 1 {
            for end in start..=expected.len() {
                assert_eq!(
                    env.defuse
                        .mt_tokens(from_token + start..from_token + end)
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
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        assert_eq!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap(),
            expected[..]
        );

        for i in 0..=3 {
            assert_eq!(
                env.defuse
                    .mt_tokens_for_owner(user1.id(), i..)
                    .await
                    .unwrap(),
                expected[i..]
            );
        }

        for i in 0..=3 {
            assert_eq!(
                env.defuse
                    .mt_tokens_for_owner(user1.id(), ..i)
                    .await
                    .unwrap(),
                expected[..i]
            );
        }

        for i in 1..=3 {
            assert_eq!(
                env.defuse
                    .mt_tokens_for_owner(user1.id(), 1..i)
                    .await
                    .unwrap(),
                expected[1..i]
            );
        }

        for i in 2..=3 {
            assert_eq!(
                env.defuse
                    .mt_tokens_for_owner(user1.id(), 2..i)
                    .await
                    .unwrap(),
                expected[2..i]
            );
        }
    }
}

#[rstest]
#[tokio::test]
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

    env.initial_ft_storage_deposit(vec![user1.id()], vec![ft1.id(), ft2.id(), ft3.id()])
        .await;

    // Check already existing tokens from persistent state if it was applied
    let existing_tokens = env.defuse.mt_tokens(..).await.unwrap();

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
        assert_eq!(env.defuse.mt_tokens(..).await.unwrap(), existing_tokens);
        assert!(
            env.defuse
                .mt_tokens_for_owner(user1.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user2.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
        assert!(
            env.defuse
                .mt_tokens_for_owner(user3.id(), ..)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(ft2.id(), 5000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(ft3.id(), 8000, user1.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(ft1.id(), 1000, user2.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(ft2.id(), 5000, user2.id(), None)
        .await
        .unwrap();

    env.defuse_ft_deposit_to(ft3.id(), 8000, user2.id(), None)
        .await
        .unwrap();

    let ft1_id = TokenId::Nep141(Nep141TokenId::new(ft1.id().clone()));
    let ft2_id = TokenId::Nep141(Nep141TokenId::new(ft2.id().clone()));
    let ft3_id = TokenId::Nep141(Nep141TokenId::new(ft3.id().clone()));

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
                ft1_id.to_string(),
                100,
                None,
                user1.id().to_string(),
            )
            .await
            .unwrap();

        user1
            .mt_transfer_call(
                env.defuse.id(),
                defuse2.id(),
                ft2_id.to_string(),
                200,
                None,
                user1.id().to_string(),
            )
            .await
            .unwrap();

        user1
            .mt_transfer_call(
                env.defuse.id(),
                defuse2.id(),
                ft3_id.to_string(),
                300,
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
#[tokio::test]
async fn mt_transfer_call_calls_mt_on_transfer_single_token(
    #[case] expectation: MtTransferCallExpectation,
) {
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
    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    // Register receiver's public key in defuse2 so it can execute intents
    receiver
        .add_public_key(
            defuse2.id(),
            &receiver.signer().get_public_key().await.unwrap().into(),
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![ft.id()],
    )
    .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    // Fund user with tokens in defuse1
    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
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
                        &defuse2,
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
        near_sdk::serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    // Check balances in defuse1 (original sender)
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[0],
        "Sender balance in defuse1 should match expected"
    );

    // Check balances in defuse2 (receiver) - token is wrapped as NEP-245
    assert_eq!(
        defuse2
            .mt_balance_of(receiver.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[0],
        "Receiver balance in defuse2 should match expected"
    );
}

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
#[tokio::test]
async fn mt_transfer_call_calls_mt_on_transfer_multi_token(
    #[case] expectation: MtTransferCallExpectation,
) {
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
    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    // Register receiver's public key in defuse2 so it can execute intents
    receiver
        .add_public_key(
            defuse2.id(),
            &receiver.signer().get_public_key().await.unwrap().into(),
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![ft1.id(), ft2.id()],
    )
    .await;

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.id().clone()));

    // Fund user with tokens in defuse1
    env.defuse_ft_deposit_to(ft1.id(), 1000, user.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(ft2.id(), 2000, user.id(), None)
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
                    &defuse2,
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
    user.mt_batch_transfer_call(
        env.defuse.id(),
        defuse2.id(),
        vec![ft1_id.to_string(), ft2_id.to_string()],
        vec![1000, 2000],
        None,
        near_sdk::serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap()
    .into_result()
    .unwrap();

    // Check balances in defuse1 (original sender)
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft1_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[0],
        "Sender balance for ft1 in defuse1 should match expected"
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft2_id.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_mt_balances[1],
        "Sender balance for ft2 in defuse1 should match expected"
    );

    // Check balances in defuse2 (receiver)
    assert_eq!(
        defuse2
            .mt_balance_of(receiver.id(), &nep245_ft1_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[0],
        "Receiver balance for ft1 in defuse2 should match expected"
    );
    assert_eq!(
        defuse2
            .mt_balance_of(receiver.id(), &nep245_ft2_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balances[1],
        "Receiver balance for ft2 in defuse2 should match expected"
    );
}

#[tokio::test]
async fn mt_transfer_call_circullar_callback() {
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

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    // Step 1: Deposit tokens to user in defuse1
    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
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

    let refund_amount = user
        .mt_transfer_call(
            env.defuse.id(),
            defuse2.id(),
            &ft_id.to_string(),
            600,
            None,
            near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        )
        .await
        .expect("mt_transfer_call should succeed");

    // The inner callback to defuse1 should succeed and keep all tokens
    assert_eq!(
        refund_amount, 600,
        "Should return 600 (amount used) since tokens were successfully deposited in circular callback"
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
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
        defuse2
            .mt_balance_of(env.defuse.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        600,
        "defuse1 should have 600 wrapped tokens in defuse2 after circular callback"
    );

    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &nep245_ft_id.to_string())
            .await
            .unwrap(),
        0,
        "User should have 0 wrapped tokens in defuse2"
    );
}

#[tokio::test]
async fn mt_transfer_call_circullar_deposit() {
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
    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    // Step 1: Deposit tokens to defuse2 in defuse1
    env.defuse_ft_deposit_to(
        ft.id(),
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
    let defuse1_ft_id: TokenId = Nep141TokenId::new(ft.id().clone()).into();

    assert_eq!(
        env.defuse
            .mt_balance_of(defuse2.id(), &defuse1_ft_id.to_string())
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
        defuse2
            .mt_balance_of(env.defuse.id(), &defuse2_nep245_ft_id.to_string())
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
        env.defuse
            .mt_balance_of(user.id(), &defuse1_defuse2_nep245_ft_id.to_string())
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

    let stub_receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    // Register stub's public key in defuse2 so it can execute intents
    stub_receiver
        .add_public_key(
            defuse2.id(),
            &stub_receiver
                .signer()
                .get_public_key()
                .await
                .unwrap()
                .into(),
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(
        vec![user.id(), stub_receiver.id()],
        vec![ft1.id(), ft2.id()],
    )
    .await;

    let transfer_amounts = [1000, 2000, 3000].map(U128::from).to_vec();
    let refund_amounts = [1000, 2000, 1000].map(U128::from).to_vec();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));
    let ft2_id = TokenId::from(Nep141TokenId::new(ft2.id().clone()));

    let nep245_ft1_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft1_id.to_string(),
    ));
    let nep245_ft2_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft2_id.to_string(),
    ));

    env.defuse_ft_deposit_to(ft1.id(), 4000, user.id(), None)
        .await
        .unwrap();
    env.defuse_ft_deposit_to(ft2.id(), 2000, user.id(), None)
        .await
        .unwrap();

    let stub_action = StubAction::ExecuteAndRefund {
        multipayload: stub_receiver
            .sign_defuse_payload_default(
                &defuse2,
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
        .mt_batch_transfer_call(
            env.defuse.id(),
            defuse2.id(),
            vec![ft1_id.to_string(), ft2_id.to_string(), ft1_id.to_string()],
            transfer_amounts.into_iter().map(|a| a.0),
            None,
            near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        )
        .await
        .unwrap();

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
        env.defuse
            .mt_balance_of(user.id(), &ft1_id.to_string())
            .await
            .unwrap(),
        2000,
        "User should have: 1000 (first refund) + 1000 (third refund capped) = 2000 of token1"
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft2_id.to_string())
            .await
            .unwrap(),
        2000,
        "User should have: 2000 (second refund) = 2000 of token2 (all refunded)"
    );
}

#[tokio::test]
async fn self_transfer_withdraw_success() {
    let env = Env::builder().build().await;
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
    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    let wrapped_token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();
    // Transfer tokens from defuse1 to defuse2 - this creates NEP-245 wrapped tokens
    user.mt_transfer_call(
        env.defuse.id(),
        defuse2.id(),
        &wrapped_token_id.to_string(),
        1000,
        None,
        user.id().to_string(), // Deposit to user in defuse2
    )
    .await
    .unwrap();

    // The token in defuse2 is wrapped as NEP-245
    let double_wrapped_token_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        wrapped_token_id.to_string(),
    ));

    // Create self-transfer withdraw action
    // This will withdraw from defuse2 back to defuse1
    // Inner token IDs are extracted from the wrapped token IDs passed in mt_transfer_call
    let action = SelfAction::Withdraw(WithdrawAction {
        token: env.defuse.id().clone(), // The external token contract (defuse1 as MT contract)
        receiver_id: user.id().clone(), // Final receiver in defuse1
        memo: Some("self-transfer withdraw".to_string()),
        msg: None,
        min_gas: None,
    });

    // Perform self-transfer on defuse2 (receiver_id = defuse2 itself)
    // Use mt_batch_transfer_call to get access to logs
    let result = user
        .mt_batch_transfer_call(
            defuse2.id(),
            defuse2.id(), // self-transfer
            vec![double_wrapped_token_id.to_string()],
            vec![500],
            None,
            near_sdk::serde_json::to_string(&action).unwrap(),
        )
        .await
        .expect("self-transfer withdraw should succeed");

    let all_logs: Vec<String> = result
        .logs()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    let _ = result.into_result().unwrap();

    // Match all emitted logs exactly
    assert_eq!(
        all_logs,
        [
            // 1. MtBurn on defuse2 - burning wrapped tokens from user
            MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
                owner_id: Cow::Borrowed(user.id().as_ref()),
                authorized_id: None,
                token_ids: Cow::Owned(vec![double_wrapped_token_id.to_string()]),
                amounts: Cow::Owned(vec![U128(500)]),
                memo: Some(Cow::Borrowed("withdraw")),
            }]))
            .to_nep297_event()
            .to_event_log(),
            // 2. MtTransfer on defuse1 - transferring from defuse2 to user
            MtEvent::MtTransfer(Cow::Owned(vec![MtTransferEvent {
                authorized_id: None,
                old_owner_id: Cow::Borrowed(defuse2.id().as_ref()),
                new_owner_id: Cow::Borrowed(user.id().as_ref()),
                token_ids: Cow::Owned(vec![wrapped_token_id.to_string()]),
                amounts: Cow::Owned(vec![U128(500)]),
                memo: Some(Cow::Borrowed("self-transfer withdraw")),
            }]))
            .to_nep297_event()
            .to_event_log(),
        ]
    );

    // Verify balance decreased in defuse2
    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &double_wrapped_token_id.to_string())
            .await
            .unwrap(),
        500,
        "User should have 500 wrapped tokens left in defuse2 after withdrawal"
    );

    // Verify tokens arrived back in defuse1
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &wrapped_token_id.to_string())
            .await
            .unwrap(),
        500,
        "User should have 500 tokens back in defuse1"
    );
}

#[tokio::test]
async fn self_transfer_insufficient_balance_fails() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    // Deploy a second defuse instance (defuse2)
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

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    // Deposit tokens to user in defuse1
    env.defuse_ft_deposit_to(ft.id(), 100, user.id(), None)
        .await
        .unwrap();

    // Transfer tokens from defuse1 to defuse2 - this creates NEP-245 wrapped tokens
    user.mt_transfer_call(
        env.defuse.id(),
        defuse2.id(),
        &ft_id.to_string(),
        100,
        None,
        user.id().to_string(),
    )
    .await
    .unwrap();

    // The token in defuse2 is wrapped as NEP-245
    let wrapped_token_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft_id.to_string(),
    ));

    // Verify user has 100 wrapped tokens in defuse2
    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &wrapped_token_id.to_string())
            .await
            .unwrap(),
        100,
        "User should have 100 wrapped tokens in defuse2"
    );

    // Create self-transfer withdraw action
    // Inner token IDs are extracted from the wrapped token IDs passed in mt_transfer_call
    let action = SelfAction::Withdraw(WithdrawAction {
        token: env.defuse.id().clone(),
        receiver_id: user.id().clone(),
        memo: None,
        msg: None,
        min_gas: None,
    });

    // Attempt self-transfer with amount exceeding balance (500 > 100)
    let result = user
        .mt_transfer_call(
            defuse2.id(),
            defuse2.id(), // self-transfer
            &wrapped_token_id.to_string(),
            500, // More than available (100)
            None,
            near_sdk::serde_json::to_string(&action).unwrap(),
        )
        .await;

    // Should fail due to insufficient balance
    assert!(
        result.is_err(),
        "Self-transfer withdraw with insufficient balance should fail"
    );

    // Verify balance unchanged in defuse2
    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &wrapped_token_id.to_string())
            .await
            .unwrap(),
        100,
        "User balance should remain unchanged after failed withdrawal"
    );
}

#[tokio::test]
async fn self_transfer_withdraw_with_partial_refund() {
    let env = Env::builder().build().await;
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    // Deploy second defuse instance
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

    // Deploy receiver stub
    let receiver_stub = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.id(), receiver_stub.id()], vec![ft.id()])
        .await;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    // Deposit 1000 tokens to user in defuse1
    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    // Transfer from defuse1 to defuse2, creating wrapped tokens for user
    user.mt_transfer_call(
        env.defuse.id(),
        defuse2.id(),
        &ft_id.to_string(),
        1000,
        None,
        user.id().to_string(),
    )
    .await
    .unwrap();

    let wrapped_token_id = TokenId::Nep245(Nep245TokenId::new(
        env.defuse.id().clone(),
        ft_id.to_string(),
    ));

    // Verify user has 1000 wrapped tokens in defuse2
    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &wrapped_token_id.to_string())
            .await
            .unwrap(),
        1000
    );

    // Create self-transfer withdraw with msg that tells stub to return 300 refund
    let stub_action = StubAction::ReturnValue(300.into());
    let action = SelfAction::Withdraw(WithdrawAction {
        token: env.defuse.id().clone(),
        receiver_id: receiver_stub.id().clone(),
        memo: None,
        msg: Some(serde_json::to_string(&stub_action).unwrap()),
        min_gas: None,
    });

    // Self-transfer withdraw 500 tokens
    user.mt_transfer_call(
        defuse2.id(),
        defuse2.id(), // self-transfer
        &wrapped_token_id.to_string(),
        500,
        None,
        serde_json::to_string(&action).unwrap(),
    )
    .await
    .unwrap();

    // Verify balances:
    // Flow:
    // 1. defuse2 burns 500 wrapped tokens from user (user has 500 left)
    // 2. defuse2 calls defuse1.mt_batch_transfer_call(receiver_stub, 500, msg)
    // 3. defuse1 transfers 500 from defuse2 to receiver_stub
    // 4. receiver_stub returns 300 refund → defuse1.mt_resolve_transfer refunds 300 to defuse2
    // 5. mt_batch_transfer_call returns [200] (used amounts)
    // 6. defuse2.mt_resolve_withdraw deposits 300 wrapped tokens back to user
    // Result: User has 500 + 300 = 800 wrapped tokens

    assert_eq!(
        defuse2
            .mt_balance_of(user.id(), &wrapped_token_id.to_string())
            .await
            .unwrap(),
        800,
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(receiver_stub.id(), &ft_id.to_string())
            .await
            .unwrap(),
        200,
    );
}
