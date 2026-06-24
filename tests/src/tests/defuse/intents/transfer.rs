use defuse_sandbox::{
    account::Account,
    assert_eq_defuse_event_logs,
    extensions::{
        defuse::{
            DefuseDeployerExt, DefuseExt, DefuseSignerExt, ToEventLog,
            contract::config::{DefuseConfig, RolesConfig},
            core::{
                amounts::Amounts,
                fees::{FeesConfig, Pips},
                intents::tokens::{NotifyOnTransfer, Transfer},
                token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId},
            },
        },
        mt::{Mt, MtBalanceOfArgs, MtExt},
    },
    kit::{AccountId, Gas, NearToken},
};
use defuse_test_utils::wasms::{DEFUSE_WASM, MT_RECEIVER_STUB_WASM};
use multi_token_receiver_stub::MTReceiverMode;
use rstest::rstest;

use crate::tests::defuse::env::{Env, env};

#[derive(Debug, Clone)]
pub struct TransferCallExpectation {
    pub mode: MTReceiverMode,
    pub intent_transfer_amount: Option<u128>,
    pub expected_sender_balance: u128,
    pub expected_receiver_balance: u128,
}

#[rstest]
#[tokio::test]
async fn transfer_intent(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: other_user_id.clone(),
        tokens: Amounts::new(
            std::iter::once((
                TokenId::from(Nep141TokenId::new(ft.contract_id().clone())),
                1000,
            ))
            .collect(),
        ),
        memo: None,
        notification: None,
    };

    let initial_transfer_payload = user
        .sign_defuse_payload_default(&env.defuse, [transfer_intent.clone()])
        .await
        .unwrap();

    let (res, _) = env
        .defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [initial_transfer_payload.clone()],
        )
        .await
        .unwrap();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        0
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: &other_user_id,
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1000
    );

    assert_eq_defuse_event_logs!(initial_transfer_payload.to_event_log(), res.logs());
}

#[rstest]
#[tokio::test]
async fn transfer_intent_to_defuse(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());
    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.contract_id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.account_id().clone(),
                },
                roles: RolesConfig::default(),
            },
            DEFUSE_WASM.clone(),
        )
        .await;

    env.initial_ft_storage_deposit(
        vec![user.account_id(), defuse2.account_id()],
        vec![ft.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    // large gas limit
    {
        let transfer_intent = Transfer {
            receiver_id: defuse2.account_id().clone(),
            tokens: Amounts::new(
                std::iter::once((
                    TokenId::from(Nep141TokenId::new(ft.contract_id().clone())),
                    1000,
                ))
                .collect(),
            ),
            memo: None,
            notification: NotifyOnTransfer::new(other_user_id.to_string())
                .with_min_gas(Gas::from_tgas(500))
                .into(),
        };

        let transfer_payload = user
            .sign_defuse_payload_default(&env.defuse, [transfer_intent])
            .await
            .unwrap();

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [transfer_payload])
            .await
            .expect_err("Exceeded the prepaid gas");
    }

    // Should pass default gas limit in case of low gas
    {
        let transfer_intent = Transfer {
            receiver_id: defuse2.account_id().clone(),
            tokens: Amounts::new(
                std::iter::once((
                    TokenId::from(Nep141TokenId::new(ft.contract_id().clone())),
                    1000,
                ))
                .collect(),
            ),
            memo: None,
            notification: NotifyOnTransfer::new(other_user_id.to_string())
                .with_min_gas(Gas::from_tgas(1))
                .into(),
        };

        let transfer_payload = user
            .sign_defuse_payload_default(&env.defuse, [transfer_intent])
            .await
            .unwrap();

        assert!(
            env.mt_tokens_for_owner(defuse2.account_id(), &other_user_id, ..)
                .await
                .unwrap()
                .is_empty()
        );

        let (res, _) = env
            .defuse_simulate_and_execute_intents(
                env.defuse.contract_id(),
                [transfer_payload.clone()],
            )
            .await
            .unwrap();

        assert_eq_defuse_event_logs!(transfer_payload.to_event_log(), res.logs());

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft1.to_string(),
                })
                .await
                .unwrap()
                .0,
            0
        );

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: defuse2.account_id(),
                    token_id: &ft1.to_string(),
                })
                .await
                .unwrap()
                .0,
            1000
        );

        assert_eq!(
            env.mt_tokens(defuse2.account_id(), ..).await.unwrap().len(),
            1
        );
        assert_eq!(
            env.mt_tokens_for_owner(defuse2.account_id(), &other_user_id, ..)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(ft.balance_of(defuse2.account_id()).await.unwrap().is_zero());

        let defuse_ft1: TokenId =
            Nep245TokenId::new(env.defuse.contract_id().clone(), ft1.to_string()).into();

        assert_eq!(
            env.contract::<Mt>(defuse2.account_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &other_user_id,
                    token_id: &defuse_ft1.to_string()
                })
                .await
                .unwrap()
                .0,
            1000
        );

        assert_eq!(
            ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
            1000
        );

        assert!(ft.balance_of(defuse2.account_id()).await.unwrap().is_zero());
    }
}

#[rstest]
#[trace]
#[case::nothing_to_refund(TransferCallExpectation {
    mode: MTReceiverMode::AcceptAll,
    intent_transfer_amount: Some(1_000),
    expected_sender_balance: 0,
    expected_receiver_balance: 1_000,
})]
#[case::partial_refund(TransferCallExpectation {
    mode: MTReceiverMode::ReturnValue(300.into()),
    intent_transfer_amount: Some(1_000),
    expected_sender_balance: 300,
    expected_receiver_balance: 700,
})]
#[case::malicious_refund(TransferCallExpectation {
    mode: MTReceiverMode::ReturnValue(2_000.into()),
    intent_transfer_amount: Some(1_000),
    expected_sender_balance: 1_000,
    expected_receiver_balance: 0,
})]
#[case::receiver_panics(TransferCallExpectation {
    mode: MTReceiverMode::Panic,
    intent_transfer_amount: Some(1_000),
    expected_sender_balance: 1000,
    expected_receiver_balance: 0,
})]
#[case::malicious_receiver(TransferCallExpectation {
    mode: MTReceiverMode::LargeReturn,
    intent_transfer_amount: Some(1_000),
    expected_sender_balance: 1000,
    expected_receiver_balance: 0,
})]
#[tokio::test]
async fn transfer_intent_with_msg_to_receiver_smc(
    #[notrace]
    #[future(awt)]
    env: Env,
    #[case] expectation: TransferCallExpectation,
) {
    let initial_amount = expectation
        .intent_transfer_amount
        .expect("Transfer amount should be specified");

    let (user, ft) = futures::join!(env.create_user(), env.create_token());
    let mt_receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    env.defuse_ft_deposit_to(ft.contract_id(), initial_amount, user.account_id(), None)
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    let msg = serde_json::to_string(&expectation.mode).unwrap();

    let transfer_intent = Transfer {
        receiver_id: mt_receiver.account_id().clone(),
        tokens: Amounts::new(
            std::iter::once((
                TokenId::from(Nep141TokenId::new(ft.contract_id().clone())),
                initial_amount,
            ))
            .collect(),
        ),
        memo: None,
        notification: NotifyOnTransfer::new(msg).into(),
    };

    let transfer_payload = user
        .sign_defuse_payload_default(&env.defuse, [transfer_intent])
        .await
        .unwrap();

    let (res, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [transfer_payload.clone()])
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(transfer_payload.to_event_log(), res.logs());

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft1.to_string(),
            })
            .await
            .unwrap()
            .0,
        expectation.expected_sender_balance
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: mt_receiver.account_id(),
                token_id: &ft1.to_string(),
            })
            .await
            .unwrap()
            .0,
        expectation.expected_receiver_balance
    );
}
