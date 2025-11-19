pub mod traits;

use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;
use crate::{
    tests::{
        defuse::env::{Env, MT_RECEIVER_STUB_WASM},
        poa::factory::PoAFactoryExt,
    },
    utils::{acl::AclExt, ft::FtExt, mt::MtExt},
};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;

use defuse::{
    contract::Role,
    core::intents::tokens::FtWithdraw,
    tokens::{DepositMessage, DepositMessageActionV2, DepositMessageV2, ExecuteIntents},
};
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_sdk::{json_types::U128, serde_json};
use rstest::rstest;

#[derive(Debug, Clone)]
struct TransferCallExpectation {
    action: StubAction,
    intent_transfer_amount: Option<u128>,
    refund_if_fails: bool,
    expected_sender_ft_balance: u128,
    expected_receiver_mt_balance: u128,
}

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        user.defuse_ft_withdraw(env.defuse.id(), &ft, user.id(), 1000, None, None)
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);
}

#[tokio::test]
#[rstest]
async fn poa_deposit(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        Some(DepositMessage::new(user.id().clone()).to_string()),
        None,
    )
    .await
    .unwrap();

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw_intent(#[values(false, true)] no_registration: bool) {
    use crate::tests::defuse::{DefuseSignerExt, tokens::nep141::traits::DefuseFtReceiver};

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![&ft])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let withdraw_intent_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
                receiver_id: other_user.id().clone(),
                amount: U128(600),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            &ft,
            1000,
            DepositMessage::V2(DepositMessageV2 {
                receiver_id: user.id().clone(),
                action: Some(DepositMessageActionV2::Execute(ExecuteIntents {
                    execute_intents: [withdraw_intent_payload].into(),
                    // another promise will be created for `execute_intents()`
                    refund_if_fails: false,
                })),
            }),
        )
        .await
        .unwrap(),
        1000
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        400
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), other_user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        600
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw_intent_refund(#[values(false, true)] no_registration: bool) {
    use crate::tests::defuse::{DefuseSignerExt, tokens::nep141::traits::DefuseFtReceiver};

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let overflow_withdraw_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
                receiver_id: user.id().clone(),
                amount: U128(1001),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            &ft,
            1000,
            DepositMessage::V2(DepositMessageV2 {
                receiver_id: user.id().clone(),
                action: Some(DepositMessageActionV2::Execute(ExecuteIntents {
                    execute_intents: [overflow_withdraw_payload].into(),
                    refund_if_fails: true,
                })),
            }),
        )
        .await
        .unwrap(),
        0
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);
}

#[tokio::test]
#[rstest]
async fn ft_force_withdraw(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    other_user
        .defuse_ft_force_withdraw(
            env.defuse.id(),
            user.id(),
            &ft,
            other_user.id(),
            1000,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        0
    );

    env.acl_grant_role(
        env.defuse.id(),
        Role::UnrestrictedWithdrawer,
        other_user.id(),
    )
    .await
    .unwrap();

    assert_eq!(
        other_user
            .defuse_ft_force_withdraw(
                env.defuse.id(),
                user.id(),
                &ft,
                other_user.id(),
                1000,
                None,
                None,
            )
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        1000
    );
}

#[tokio::test]
#[rstest]
#[case::nothing_to_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 0,
    expected_receiver_mt_balance: 1_000,
})]
#[case::partial_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(300.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 300,
    expected_receiver_mt_balance: 700,
})]
#[case::malicious_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(2_000.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1_000,
    expected_receiver_mt_balance: 0,
})]
#[case::receiver_panics_results_with_no_refund(TransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 0,
})]
#[case::malicious_receiver(TransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 0,
})]
async fn ft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: TransferCallExpectation,
) {
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};

    use crate::tests::defuse::DefuseSignerExt;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(false)
        .build()
        .await;

    let (user, receiver, intent_receiver, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));
    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![&ft],
    )
    .await;

    let root = env.sandbox().root_account();

    root.ft_transfer(&ft, user.id(), 1000, None).await.unwrap();
    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);

    let intents = match &expectation.intent_transfer_amount {
        Some(amount) => vec![
            receiver
                .sign_defuse_payload_default(
                    env.defuse.id(),
                    [Transfer {
                        receiver_id: intent_receiver.id().clone(),
                        tokens: Amounts::new(std::iter::once((ft_id.clone(), *amount)).collect()),
                        memo: None,
                        notification: None,
                    }],
                )
                .await
                .unwrap(),
        ],
        None => vec![],
    };

    let deposit_message = if intents.is_empty() {
        DepositMessage::V2(DepositMessageV2 {
            receiver_id: receiver.id().clone(),
            action: Some(DepositMessageActionV2::Notify(
                defuse::core::intents::tokens::NotifyOnTransfer {
                    msg: serde_json::to_string(&expectation.action).unwrap(),
                    min_gas: None,
                },
            )),
        })
    } else {
        DepositMessage::V2(DepositMessageV2 {
            receiver_id: receiver.id().clone(),
            action: Some(DepositMessageActionV2::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        })
    };

    user.ft_transfer_call(
        &ft,
        env.defuse.id(),
        1000,
        None,
        &serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        env.ft_token_balance_of(&ft, user.id()).await.unwrap(),
        expectation.expected_sender_ft_balance
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), receiver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balance
    );
}

#[tokio::test]
async fn ft_transfer_call_with_intent_transfers_from_initial_user_balance() {
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};

    use crate::tests::defuse::DefuseSignerExt;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(false)
        .build()
        .await;

    let (user, receiver, intent_receiver, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    // Deploy the stub receiver contract
    receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));
    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![&ft],
    )
    .await;

    // Step 1: Deposit 1000 tokens to user's MT balance
    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );

    // Give user 1000 FT tokens for the ft_transfer_call
    let root = env.sandbox().root_account();
    root.ft_transfer(&ft, user.id(), 1000, None).await.unwrap();
    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);

    // Step 2: Create intent to transfer 500 tokens from user to intent_receiver
    let transfer_intent = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [Transfer {
                receiver_id: intent_receiver.id().clone(),
                tokens: Amounts::new(std::iter::once((ft_id.clone(), 500)).collect()),
                memo: None,
                notification: None,
            }],
        )
        .await
        .unwrap();

    // Step 3: ft_transfer_call with 1000 tokens and intent to transfer 500 tokens
    let deposit_message = DepositMessage::V2(DepositMessageV2 {
        receiver_id: receiver.id().clone(),
        action: Some(DepositMessageActionV2::Execute(ExecuteIntents {
            execute_intents: vec![transfer_intent],
            refund_if_fails: true,
        })),
    });

    user.ft_transfer_call(
        &ft,
        env.defuse.id(),
        1000,
        None,
        &serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    // Step 4: Verify final balances
    // User FT balance should be 0 (all deposited, no refund since intent succeeded)
    assert_eq!(
        env.ft_token_balance_of(&ft, user.id()).await.unwrap(),
        0,
        "User FT balance should be 0 after successful deposit and intent execution"
    );

    // User's MT balance should be 500 (1000 initial - 500 transferred via intent)
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        500,
        "User's MT balance should be 500 after intent transfer"
    );

    // Receiver should have 1000 MT balance (from the ft_transfer_call deposit)
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), receiver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000,
        "Receiver's MT balance should be 1000 from deposit"
    );

    // intent_receiver should have 500 MT tokens from the intent transfer
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), intent_receiver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        500,
        "Intent receiver should have 500 MT tokens"
    );
}
