use std::borrow::Cow;

use crate::{
    env::{DEFUSE_WASM, TransferCallExpectation},
    extensions::defuse::{deployer::DefuseExt, tokens::imt::DefuseImtMinter},
};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::{
        accounts::AccountEvent,
        amounts::Amounts,
        events::{DefuseEvent, MaybeIntentEvent},
        fees::FeesConfig,
        intents::tokens::NotifyOnTransfer,
        token_id::TokenId,
        tokens::{
            MAX_TOKEN_ID_LEN,
            imt::{ImtMintEvent, ImtTokens},
        },
    },
};
use defuse_sandbox::assert_a_contains_b;

use defuse_escrow_swap::{
    Pips,
    token_id::{imt::ImtTokenId, nep245::Nep245TokenId},
};
use defuse_nep245::{MtEvent, MtMintEvent};
use defuse_sandbox::{FnCallBuilder, extensions::mt::MtViewExt};
use defuse_test_utils::asserts::ResultAssertsExt;
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{AccountId, AsNep297Event, Gas, NearToken, json_types::U128};
use rstest::rstest;

use crate::env::{Env, MT_RECEIVER_STUB_WASM};

#[rstest]
#[tokio::test]
async fn imt_mint_call() {
    let env = Env::builder().create_unique_users().build().await;

    let user = env.create_user().await;
    let token = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;
    let receiver_id = "imt_tokens_receiver".parse::<AccountId>().unwrap();

    let (minted_tokens, result) = user
        .imt_mint(
            env.defuse.id(),
            receiver_id.clone(),
            [(token.clone(), amount)],
            Some(memo.to_string()),
            None,
        )
        .await
        .unwrap();

    let imt_id = TokenId::from(ImtTokenId::new(user.id().clone(), token.to_string()));

    assert_eq!(
        env.defuse
            .mt_balance_of(receiver_id.clone(), &imt_id.to_string())
            .await
            .unwrap(),
        amount
    );

    assert_eq!(minted_tokens.get(&imt_id), Some(&amount));

    let events = [
        MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
            owner_id: user.id().into(),
            token_ids: vec![imt_id.to_string()].into(),
            amounts: vec![U128::from(amount)].into(),
            memo: Some(memo.into()),
        }]))
        .to_nep297_event()
        .to_event_log(),
        DefuseEvent::ImtMint(
            vec![MaybeIntentEvent::direct(AccountEvent {
                account_id: user.id().clone().into(),
                event: ImtMintEvent {
                    receiver_id: Cow::Owned(receiver_id),
                    tokens: ImtTokens::from(Amounts::new(
                        std::iter::once((token.clone(), amount)).collect(),
                    )),
                    memo: Cow::Owned(Some(memo.to_string())),
                },
            })]
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
async fn failed_imt_mint_call() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token = ["a"; MAX_TOKEN_ID_LEN + 1].join("");
    let amount = 1000;
    let receiver_id = "imt_tokens_receiver".parse::<AccountId>().unwrap();

    user.imt_mint(
        env.defuse.id(),
        receiver_id.clone(),
        [(token.clone(), amount)],
        None,
        None,
    )
    .await
    .assert_err_contains("token_id is too long");
}

#[rstest]
#[trace]
#[tokio::test]
async fn imt_mint_call_to_defuse() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;
    let amount = 1000;
    let receiver_id = "imt_tokens_receiver".parse::<AccountId>().unwrap();

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
        user.imt_mint(
            env.defuse.id(),
            defuse2.id(),
            [(ft.clone(), amount)],
            None,
            Some(NotifyOnTransfer::new(receiver_id.to_string()).with_min_gas(Gas::from_tgas(500))),
        )
        .await
        .expect_err("Exceeded the prepaid gas");
    }

    // Should pass default gas limit in case of low gas
    {
        assert!(defuse2.mt_tokens(..).await.unwrap().is_empty());

        user.imt_mint(
            env.defuse.id(),
            defuse2.id(),
            [(ft.clone(), amount)],
            None,
            Some(NotifyOnTransfer::new(receiver_id.to_string()).with_min_gas(Gas::from_tgas(1))),
        )
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
                .mt_tokens_for_owner(&receiver_id, ..)
                .await
                .unwrap()
                .len(),
            1
        );

        let defuse_ft1: TokenId =
            Nep245TokenId::new(env.defuse.id().clone(), mt_token.to_string()).into();

        assert_eq!(
            defuse2
                .mt_balance_of(&receiver_id, &defuse_ft1.to_string())
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
    transfer_amount: Some(1_000),
    expected_sender_balance: 0,
    expected_receiver_balance: 1_000,
})]
#[case::partial_refund(TransferCallExpectation {
    mode: MTReceiverMode::ReturnValue(300.into()),
    transfer_amount: Some(1_000),
    expected_sender_balance: 300,
    expected_receiver_balance: 700,
})]
#[case::malicious_refund(TransferCallExpectation {
    mode: MTReceiverMode::ReturnValue(2_000.into()),
    transfer_amount: Some(1_000),
    expected_sender_balance: 1_000,
    expected_receiver_balance: 0,
})]
#[case::receiver_panics(TransferCallExpectation {
    mode: MTReceiverMode::Panic,
    transfer_amount: Some(1_000),
    expected_sender_balance: 1000,
    expected_receiver_balance: 0,
})]
#[case::malicious_receiver(TransferCallExpectation {
    mode: MTReceiverMode::LargeReturn,
    transfer_amount: Some(1_000),
    expected_sender_balance: 1000,
    expected_receiver_balance: 0,
})]
#[tokio::test]
async fn mt_mint_call_with_msg_to_receiver_smc(#[case] expectation: TransferCallExpectation) {
    let initial_amount = expectation
        .transfer_amount
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

    user.imt_mint(
        env.defuse.id(),
        mt_receiver.id(),
        [(ft1.clone(), initial_amount)],
        None,
        Some(NotifyOnTransfer::new(msg)),
    )
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
