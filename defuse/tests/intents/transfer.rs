use super::ExecuteIntentsExt;
use crate::{
    DefuseSignerExt,
    env::{Env, TransferCallExpectation},
};
use defuse::core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::{FeesConfig, Pips},
};
use defuse_contract_extensions::defuse::deployer::DefuseExt;
use defuse_sandbox::extensions::ft::FtViewExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{AccountId, Gas, serde_json};
use rstest::rstest;

use defuse::core::amounts::Amounts;

#[rstest]
#[trace]
#[tokio::test]
async fn transfer_intent() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: other_user_id.clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft.id().clone())), 1000)).collect(),
        ),
        memo: None,
        notification: None,
    };

    let initial_transfer_payload = user
        .sign_defuse_payload_default(&env.defuse, [transfer_intent])
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [initial_transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(&other_user_id, &token_id.to_string())
            .await
            .unwrap(),
        1000
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn transfer_intent_to_defuse() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());
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
            false,
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.id(), defuse2.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    // large gas limit
    {
        let transfer_intent = Transfer {
            receiver_id: defuse2.id().clone(),
            tokens: Amounts::new(
                std::iter::once((TokenId::from(Nep141TokenId::new(ft.id().clone())), 1000))
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

        env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
            .await
            .expect_err("Exceeded the prepaid gas");
    }

    // Should pass default gas limit in case of low gas
    {
        let transfer_intent = Transfer {
            receiver_id: defuse2.id().clone(),
            tokens: Amounts::new(
                std::iter::once((TokenId::from(Nep141TokenId::new(ft.id().clone())), 1000))
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

        assert!(defuse2.mt_tokens(..).await.unwrap().is_empty());

        env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &ft1.to_string())
                .await
                .unwrap(),
            0
        );

        assert_eq!(
            env.defuse
                .mt_balance_of(defuse2.id(), &ft1.to_string())
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
        assert_eq!(ft.ft_balance_of(defuse2.id()).await.unwrap(), 0);

        let defuse_ft1: TokenId =
            Nep245TokenId::new(env.defuse.id().clone(), ft1.to_string()).into();

        assert_eq!(
            defuse2
                .mt_balance_of(&other_user_id, &defuse_ft1.to_string())
                .await
                .unwrap(),
            1000
        );

        assert_eq!(ft.ft_balance_of(env.defuse.id()).await.unwrap(), 1000);

        assert_eq!(ft.ft_balance_of(defuse2.id()).await.unwrap(), 0);
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
async fn transfer_intent_with_msg_to_receiver_smc(#[case] expectation: TransferCallExpectation) {
    let initial_amount = expectation
        .intent_transfer_amount
        .expect("Transfer amount should be specified");

    let env = Env::builder().build().await;

    let (user, ft, mt_receiver) = futures::join!(
        env.create_user(),
        env.create_token(),
        env.deploy_mt_receiver_stub()
    );

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), initial_amount, user.id(), None)
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    let msg = serde_json::to_string(&expectation.mode).unwrap();

    let transfer_intent = Transfer {
        receiver_id: mt_receiver.id().clone(),
        tokens: Amounts::new(
            std::iter::once((
                TokenId::from(Nep141TokenId::new(ft.id().clone())),
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

    env.simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft1.to_string())
            .await
            .unwrap(),
        expectation.expected_sender_balance
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(mt_receiver.id(), &ft1.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_balance
    );
}
