use defuse::core::intents::tokens::FtWithdraw;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::sandbox_ext::{
    deployer::DefuseExt, intents::ExecuteIntentsExt, tokens::nep141::DefuseFtDepositor,
};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::{FeesConfig, Pips},
};
use defuse_sandbox::extensions::ft::FtViewExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::extensions::wnear::WNearExt;
use defuse_test_utils::asserts::ResultAssertsExt;
use near_sdk::{AccountId, Gas, NearToken};
use rstest::rstest;

use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn ft_withdraw_intent() {
    use crate::tests::defuse::DefuseSignerExt;

    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    {
        env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &token_id.to_string())
                .await
                .unwrap(),
            1000
        );
    }

    let initial_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: other_user_id.clone(),
                amount: 1000.into(),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [initial_withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &token_id.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(ft.ft_balance_of(&other_user_id).await.unwrap(), 0);

    let missing_storage_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: other_user_id.clone(),
                amount: 1000.into(),
                memo: None,
                msg: None,
                // user has no wnear yet
                storage_deposit: Some(STORAGE_DEPOSIT),
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [missing_storage_payload])
        .await
        .unwrap_err();

    // send user some near
    env.tx(user.id()).transfer(STORAGE_DEPOSIT).await.unwrap();

    // wrap NEAR
    user.near_deposit(env.wnear.id(), STORAGE_DEPOSIT)
        .await
        .unwrap();

    // deposit wNEAR
    user.defuse_ft_deposit(
        env.defuse.id(),
        env.wnear.id(),
        STORAGE_DEPOSIT.as_yoctonear(),
        None,
    )
    .await
    .unwrap();

    let old_defuse_balance = env.defuse.view().await.unwrap().amount;

    // too large min_gas specified
    let too_large_min_gas_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: other_user_id.clone(),
                amount: 1000.into(),
                memo: None,
                msg: None,
                storage_deposit: Some(STORAGE_DEPOSIT),
                min_gas: Some(Gas::from_tgas(300)),
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [too_large_min_gas_payload])
        .await
        .assert_err_contains("Exceeded the prepaid gas.");

    let valid_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: other_user_id.clone(),
                amount: 1000.into(),
                memo: None,
                msg: None,
                storage_deposit: Some(STORAGE_DEPOSIT),
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [valid_payload])
        .await
        .unwrap();
    let new_defuse_balance = env.defuse.view().await.unwrap().amount;

    assert!(
        new_defuse_balance >= old_defuse_balance,
        "contract balance must not decrease"
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    // The storage deposit consumed the wNEAR balance
    assert_eq!(
        env.defuse
            .mt_balance_of(
                user.id(),
                &TokenId::from(Nep141TokenId::new(env.wnear.id().clone())).to_string()
            )
            .await
            .unwrap(),
        0,
    );

    assert_eq!(ft.ft_balance_of(&other_user_id).await.unwrap(), 1000);
}

#[rstest]
#[trace]
#[tokio::test]
async fn ft_withdraw_intent_msg() {
    use crate::tests::defuse::DefuseSignerExt;

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

    // too small min_gas
    {
        let low_min_gas_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [FtWithdraw {
                    token: ft.id().clone(),
                    receiver_id: defuse2.id().clone(),
                    amount: 400.into(),
                    memo: Some("defuse-to-defuse".to_string()),
                    msg: Some(other_user_id.to_string()),
                    storage_deposit: None,
                    // too small, but minimum of 30TGas will be used
                    min_gas: Some(Gas::from_tgas(1)),
                }],
            )
            .await
            .unwrap();

        env.simulate_and_execute_intents(env.defuse.id(), [low_min_gas_payload])
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &ft1.to_string())
                .await
                .unwrap(),
            600
        );

        assert_eq!(ft.ft_balance_of(env.defuse.id()).await.unwrap(), 600);

        assert_eq!(ft.ft_balance_of(defuse2.id()).await.unwrap(), 400);
        assert_eq!(
            defuse2
                .mt_balance_of(&other_user_id, &ft1.to_string())
                .await
                .unwrap(),
            400
        );
    }

    let remaining_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: defuse2.id().clone(),
                amount: 600.into(),
                memo: Some("defuse-to-defuse".to_string()),
                msg: Some(other_user_id.to_string()),
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [remaining_withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(ft.ft_balance_of(env.defuse.id()).await.unwrap(), 0);

    assert_eq!(ft.ft_balance_of(defuse2.id()).await.unwrap(), 1000);
    assert_eq!(
        defuse2
            .mt_balance_of(&other_user_id, &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
}
