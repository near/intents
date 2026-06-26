use defuse_sandbox::{
    account::Account,
    extensions::{
        DEFAULT_GAS,
        acl::AccessControllableExt,
        defuse::{
            DefuseExt, DefuseSignerExt,
            contract::Role,
            core::{
                amounts::Amounts,
                intents::tokens::{FtWithdraw, NotifyOnTransfer, Transfer},
                token_id::{TokenId, nep141::Nep141TokenId},
            },
            tokens::{DepositAction, DepositMessage, ExecuteIntents},
        },
        mt::{Mt, MtBalanceOfArgs},
        poa::PoAFactoryExt,
    },
    kit::{Final, NearToken},
};
use defuse_test_utils::wasms::MT_RECEIVER_STUB_WASM;
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_sdk::json_types::U128;
use rstest::rstest;

use crate::tests::defuse::env::{Env, env};

#[rstest]
#[tokio::test]
async fn deposit_withdraw(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        1000
    );

    assert_eq!(
        user.defuse_ft_withdraw(
            env.defuse.contract_id(),
            ft.contract_id(),
            user.account_id(),
            1000,
            None,
            None,
        )
        .await
        .unwrap()
        .1,
        1000
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );

    assert_eq!(ft.balance_of(user.account_id()).await.unwrap().raw(), 1000);
}

#[rstest]
#[tokio::test]
async fn poa_deposit(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.contract_id(),
        &env.poa_factory.ft_name(ft.contract_id()),
        user.account_id(),
        1000,
        Some(DepositMessage::new(user.account_id().clone()).to_string()),
        None,
    )
    .await
    .unwrap();

    assert_eq!(ft.balance_of(user.account_id()).await.unwrap().raw(), 0);
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );
}

#[rstest]
#[tokio::test]
async fn deposit_withdraw_intent(#[future(awt)] env: Env) {
    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![user.account_id(), other_user.account_id()],
        vec![ft.contract_id()],
    )
    .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.contract_id(),
        &env.poa_factory.ft_name(ft.contract_id()),
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let withdraw_intent_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
                receiver_id: other_user.account_id().clone(),
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
        user.ft(ft.contract_id())
            .unwrap()
            .transfer_call(
                env.defuse.contract_id(),
                1000u128,
                DepositMessage {
                    receiver_id: user.account_id().clone(),
                    action: Some(DepositAction::Execute(ExecuteIntents {
                        execute_intents: [withdraw_intent_payload].into(),
                        // another promise will be created for `execute_intents()`
                        refund_if_fails: false,
                    })),
                }
                .to_string()
            )
            .await
            .unwrap()
            .json::<U128>()
            .unwrap()
            .0,
        1000
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    assert_eq!(ft.balance_of(user.account_id()).await.unwrap().raw(), 0);
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        400
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: other_user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );
    assert_eq!(
        ft.balance_of(other_user.account_id()).await.unwrap().raw(),
        600
    );
}

#[rstest]
#[tokio::test]
async fn deposit_withdraw_intent_refund(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.contract_id(),
        &env.poa_factory.ft_name(ft.contract_id()),
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let overflow_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
                receiver_id: user.account_id().clone(),
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
        user.ft(ft.contract_id())
            .unwrap()
            .transfer_call(
                env.defuse.contract_id(),
                1000u128,
                DepositMessage {
                    receiver_id: user.account_id().clone(),
                    action: Some(DepositAction::Execute(ExecuteIntents {
                        execute_intents: [overflow_withdraw_payload].into(),
                        refund_if_fails: true,
                    })),
                }
                .to_string()
            )
            .await
            .unwrap()
            .json::<U128>()
            .unwrap()
            .0,
        0
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );
    assert_eq!(ft.balance_of(user.account_id()).await.unwrap().raw(), 1000);
}

#[rstest]
#[tokio::test]
async fn ft_force_withdraw(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![user.account_id(), other_user.account_id()],
        vec![ft.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    other_user
        .defuse_ft_force_withdraw(
            env.defuse.contract_id(),
            user.account_id(),
            ft.contract_id(),
            other_user.account_id(),
            1000,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        1000
    );
    assert_eq!(
        ft.balance_of(other_user.account_id()).await.unwrap().raw(),
        0
    );

    env.acl_grant_role(
        env.defuse.contract_id(),
        Role::UnrestrictedWithdrawer,
        other_user.account_id(),
    )
    .await
    .unwrap();

    assert_eq!(
        other_user
            .defuse_ft_force_withdraw(
                env.defuse.contract_id(),
                user.account_id(),
                ft.contract_id(),
                other_user.account_id(),
                1000,
                None,
                None,
            )
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );
    assert_eq!(
        ft.balance_of(other_user.account_id()).await.unwrap().raw(),
        1000
    );
}

#[derive(Debug, Clone)]
struct TransferCallExpectation {
    action: StubAction,
    intent_transfer_amount: Option<u128>,
    refund_if_fails: bool,
    expected_sender_ft_balance: u128,
    expected_receiver_mt_balance: u128,
}

#[rstest]
#[case::nothing_to_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 0,
    expected_receiver_mt_balance: 1_500,
})]
#[case::partial_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(300.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 300,
    expected_receiver_mt_balance: 1_200,
})]
#[case::malicious_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(2_000.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1_000,
    expected_receiver_mt_balance: 500,
})]
#[case::receiver_panics_results_with_no_refund(TransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 500,
})]
#[case::malicious_receiver(TransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 500,
})]
#[tokio::test]
async fn ft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: TransferCallExpectation,
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (user, intent_receiver, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));
    env.initial_ft_storage_deposit(
        vec![
            user.account_id(),
            receiver.account_id(),
            intent_receiver.account_id(),
        ],
        vec![ft.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft.contract_id(), 500, receiver.account_id(), None)
        .await
        .unwrap();

    env.ft(ft.contract_id())
        .unwrap()
        .transfer(user.account_id(), 1000u128)
        .await
        .unwrap()
        .result()
        .unwrap();

    assert_eq!(ft.balance_of(user.account_id()).await.unwrap().raw(), 1000);

    let intents = match &expectation.intent_transfer_amount {
        Some(amount) => vec![
            receiver
                .sign_defuse_payload_default(
                    &env.defuse,
                    [Transfer {
                        receiver_id: intent_receiver.account_id().clone(),
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
        DepositMessage {
            receiver_id: receiver.account_id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.account_id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    user.ft(ft.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            1000u128,
            deposit_message.to_string(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap()
        .result()
        .unwrap();

    assert_eq!(
        ft.balance_of(user.account_id()).await.unwrap().raw(),
        expectation.expected_sender_ft_balance
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: receiver.account_id(),
                token_id: &ft_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        expectation.expected_receiver_mt_balance
    );
}
