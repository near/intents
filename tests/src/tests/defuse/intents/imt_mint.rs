use defuse_fees::Pips;
use defuse_sandbox::{
    account::Account,
    assert_eq_defuse_event_logs,
    extensions::{
        defuse::{
            DefuseDeployerExt, DefuseExt, DefuseSignerExt, ToEventLog,
            contract::config::{DefuseConfig, RolesConfig},
            core::{
                amounts::Amounts,
                fees::FeesConfig,
                intents::{imt::ImtMint, tokens::NotifyOnTransfer},
                token_id::{TokenId, imt::ImtTokenId, nep245::Nep245TokenId},
                tokens::MAX_TOKEN_ID_LEN,
            },
        },
        mt::{Mt, MtBalanceOfArgs, MtExt},
    },
};
use defuse_test_utils::asserts::ResultAssertsExt;
use multi_token_receiver_stub::MTReceiverMode;
use rstest::rstest;

use near_sdk::{AccountId, Gas, NearToken};

use crate::tests::defuse::{
    env::{Env, env},
    intents::transfer::TransferCallExpectation,
};
use defuse_test_utils::wasms::{DEFUSE_WASM, MT_RECEIVER_STUB_WASM};

#[rstest]
#[tokio::test]
async fn imt_mint_intent(#[future(awt)] env: Env) {
    let user = env.create_user().await;

    let token = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
        receiver_id: user.account_id().clone(),
        notification: None,
    };
    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let (result, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload.clone()])
        .await
        .unwrap();

    let mt_id = TokenId::from(ImtTokenId::new(user.account_id().clone(), token.clone()));

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &mt_id.to_string()
            })
            .await
            .unwrap()
            .0,
        amount
    );

    assert_eq_defuse_event_logs!(mint_payload.to_event_log(), result.logs());
}

#[rstest]
#[tokio::test]
async fn failed_imt_mint_intent(#[future(awt)] env: Env) {
    let user = env.create_user().await;

    let token = ["a"; MAX_TOKEN_ID_LEN + 1].join("");
    let amount = 1000;

    let intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
        memo: None,
        receiver_id: user.account_id().clone(),
        notification: None,
    };
    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload.clone()])
        .await
        .assert_err_contains("token_id is too long");
}

#[rstest]
#[tokio::test]
async fn imt_mint_intent_to_defuse(#[future(awt)] env: Env) {
    let user = env.create_user().await;
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
    let ft = "newtoken.near".to_string();

    // large gas limit
    {
        let mint_intent = ImtMint {
            receiver_id: defuse2.account_id().clone(),
            tokens: Amounts::new(std::iter::once((ft.clone(), 1000)).collect()),
            memo: None,
            notification: NotifyOnTransfer::new(other_user_id.to_string())
                .with_min_gas(Gas::from_tgas(500))
                .into(),
        };

        let transfer_payload = user
            .sign_defuse_payload_default(&env.defuse, [mint_intent])
            .await
            .unwrap();

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [transfer_payload])
            .await
            .expect_err("Exceeded the prepaid gas");
    }

    // Should pass default gas limit in case of low gas
    {
        let mint_intent = ImtMint {
            receiver_id: defuse2.account_id().clone(),
            tokens: Amounts::new(std::iter::once((ft.clone(), 1000)).collect()),
            memo: None,
            notification: NotifyOnTransfer::new(other_user_id.to_string())
                .with_min_gas(Gas::from_tgas(1))
                .into(),
        };

        let mint_payload = user
            .sign_defuse_payload_default(&env.defuse, [mint_intent])
            .await
            .unwrap();

        assert!(
            env.mt_tokens(defuse2.account_id(), ..)
                .await
                .unwrap()
                .is_empty()
        );

        let (res, _) = env
            .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload.clone()])
            .await
            .unwrap();

        assert_eq_defuse_event_logs!(mint_payload.to_event_log(), res.logs());

        let mt_token = TokenId::from(ImtTokenId::new(user.account_id().clone(), ft.clone()));

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: defuse2.account_id(),
                    token_id: &mt_token.to_string(),
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

        let defuse_ft1: TokenId =
            Nep245TokenId::new(env.defuse.contract_id().clone(), mt_token.to_string()).into();

        assert_eq!(
            env.contract::<Mt>(defuse2.account_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &other_user_id,
                    token_id: &defuse_ft1.to_string(),
                })
                .await
                .unwrap()
                .0,
            1000
        );
    }
}

#[rstest]
#[case::nothing_to_refund(TransferCallExpectation{
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
async fn imt_mint_intent_with_msg_to_receiver_smc(
    #[future(awt)] env: Env,
    #[case] expectation: TransferCallExpectation,
) {
    let initial_amount = expectation
        .intent_transfer_amount
        .expect("Transfer amount should be specified");

    let user = env.create_user().await;
    let mt_receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    let ft1 = "some-mt-token.near".to_string();

    let msg = serde_json::to_string(&expectation.mode).unwrap();

    let mint_intent = ImtMint {
        receiver_id: mt_receiver.account_id().clone(),
        tokens: Amounts::new(std::iter::once((ft1.clone(), initial_amount)).collect()),
        memo: None,
        notification: NotifyOnTransfer::new(msg).into(),
    };

    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [mint_intent])
        .await
        .unwrap();

    let (res, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload.clone()])
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(mint_payload.to_event_log(), res.logs());

    let mt_token = TokenId::from(ImtTokenId::new(user.account_id().clone(), ft1.clone()));

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &mt_token.to_string(),
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
                token_id: &mt_token.to_string(),
            })
            .await
            .unwrap()
            .0,
        expectation.expected_receiver_balance
    );
}
