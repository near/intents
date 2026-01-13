use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::amounts::Amounts;
use defuse::core::fees::{FeesConfig, Pips};
use defuse::core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::nep245::{MtBurnEvent, MtEvent, MtTransferEvent};
use defuse::tokens::DepositMessage;
use defuse::tokens::{DepositAction, ExecuteIntents};
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_sandbox::extensions::defuse::{account_manager::AccountManagerExt, deployer::DefuseExt};
use defuse_sandbox::extensions::mt::{MtExt, MtViewExt};
use defuse_sandbox::tx::FnCallBuilder;
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_sdk::{AsNep297Event, json_types::U128};
use near_sdk::{NearToken, serde_json};
use rstest::rstest;
use std::borrow::Cow;

use crate::MT_RECEIVER_STUB_WASM;
use defuse_tests::env::Env;

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

#[allow(clippy::too_many_lines)]
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
