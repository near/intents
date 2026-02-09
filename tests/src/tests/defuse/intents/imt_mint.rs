use std::borrow::Cow;

use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::accounts::AccountEvent;
use defuse::core::amounts::Amounts;
use defuse::core::crypto::Payload;
use defuse::core::events::DefuseEvent;
use defuse::core::fees::FeesConfig;
use defuse::core::intents::{
    MaybeIntentEvent,
    tokens::{NotifyOnTransfer, imt::ImtMint},
};
use defuse::core::token_id::TokenId;
use defuse::core::tokens::MAX_TOKEN_ID_LEN;
use defuse::core::tokens::imt::ImtMintEvent;
use defuse::nep245::{MtEvent, MtMintEvent};
use defuse_escrow_swap::Pips;
use defuse_escrow_swap::token_id::imt::ImtTokenId;
use defuse_escrow_swap::token_id::nep245::Nep245TokenId;
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::tx::FnCallBuilder;
use defuse_test_utils::asserts::ResultAssertsExt;
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::json_types::U128;
use rstest::rstest;

use near_sdk::{AccountId, AsNep297Event, Gas, NearToken};

use crate::env::{DEFUSE_WASM, Env, MT_RECEIVER_STUB_WASM};
use crate::extensions::defuse::deployer::DefuseExt;
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use crate::tests::defuse::intents::transfer::TransferCallExpectation;

#[rstest]
#[trace]
#[tokio::test]
async fn mt_mint_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
        receiver_id: user.id().clone(),
        notification: None,
    };
    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [mint_payload.clone()])
        .await
        .unwrap();

    let mt_id = TokenId::from(ImtTokenId::new(user.id().clone(), token.to_string()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &mt_id.to_string())
            .await
            .unwrap(),
        amount
    );

    let events = [
        MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
            owner_id: user.id().into(),
            token_ids: vec![mt_id.to_string()].into(),
            amounts: vec![U128::from(amount)].into(),
            memo: Some(memo.into()),
        }]))
        .to_nep297_event()
        .to_event_log(),
        DefuseEvent::ImtMint(
            vec![MaybeIntentEvent::new_with_hash(
                AccountEvent {
                    account_id: user.id().clone().into(),
                    event: ImtMintEvent {
                        receiver_id: Cow::Borrowed(&intent.receiver_id),
                        tokens: intent.tokens.clone(),
                        memo: Cow::Borrowed(&intent.memo),
                    },
                },
                mint_payload.hash(),
            )]
            .into(),
        )
        .to_nep297_event()
        .to_event_log(),
    ];

    assert_a_contains_b!(
        a: result.logs().clone(),
        b: events
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn failed_imt_mint_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token = ["a"; MAX_TOKEN_ID_LEN + 1].join("");
    let amount = 1000;

    let intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
        memo: None,
        receiver_id: user.id().clone(),
        notification: None,
    };
    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [mint_payload.clone()])
        .await
        .assert_err_contains("token_id is too long");
}

#[rstest]
#[trace]
#[tokio::test]
async fn mt_mint_intent_to_defuse() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;
    let other_user_id: AccountId = "other-user.near".parse().unwrap();

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
            DEFUSE_WASM.clone(),
        )
        .await
        .unwrap();

    let ft = "newtoken.near".to_string();

    // large gas limit
    {
        let mint_intent = ImtMint {
            receiver_id: defuse2.id().clone(),
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

        env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
            .await
            .expect_err("Exceeded the prepaid gas");
    }

    // Should pass default gas limit in case of low gas
    {
        let mint_intent = ImtMint {
            receiver_id: defuse2.id().clone(),
            tokens: Amounts::new(std::iter::once((ft.clone(), 1000)).collect()),
            memo: None,
            notification: NotifyOnTransfer::new(other_user_id.to_string())
                .with_min_gas(Gas::from_tgas(1))
                .into(),
        };

        let transfer_payload = user
            .sign_defuse_payload_default(&env.defuse, [mint_intent])
            .await
            .unwrap();

        assert!(defuse2.mt_tokens(..).await.unwrap().is_empty());

        env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
            .await
            .unwrap();

        let mt_token = TokenId::from(ImtTokenId::new(user.id().clone(), ft.to_string()));

        assert_eq!(
            env.defuse
                .mt_balance_of(defuse2.id(), &mt_token.to_string())
                .await
                .unwrap(),
            1000
        );

        assert_eq!(defuse2.mt_tokens(..).await.unwrap().len(), 1);
        assert_eq!(
            defuse2
                .mt_tokens_for_owner(&other_user_id, ..)
                .await
                .unwrap()
                .len(),
            1
        );

        let defuse_ft1: TokenId =
            Nep245TokenId::new(env.defuse.id().clone(), mt_token.to_string()).into();

        assert_eq!(
            defuse2
                .mt_balance_of(&other_user_id, &defuse_ft1.to_string())
                .await
                .unwrap(),
            1000
        );
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
async fn mt_mint_intent_with_msg_to_receiver_smc(#[case] expectation: TransferCallExpectation) {
    let initial_amount = expectation
        .intent_transfer_amount
        .expect("Transfer amount should be specified");

    let env = Env::builder().build().await;

    let user = env.create_user().await;
    let mt_receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    let ft1 = "some-mt-token.near".to_string();

    let msg = serde_json::to_string(&expectation.mode).unwrap();

    let mint_intent = ImtMint {
        receiver_id: mt_receiver.id().clone(),
        tokens: Amounts::new(std::iter::once((ft1.clone(), initial_amount)).collect()),
        memo: None,
        notification: NotifyOnTransfer::new(msg).into(),
    };

    let transfer_payload = user
        .sign_defuse_payload_default(&env.defuse, [mint_intent])
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    let mt_token = TokenId::from(ImtTokenId::new(user.id().clone(), ft1.to_string()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &mt_token.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_balance
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(mt_receiver.id(), &mt_token.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_balance
    );
}
