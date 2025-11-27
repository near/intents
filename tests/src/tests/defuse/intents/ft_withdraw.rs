use super::ExecuteIntentsExt;
use crate::tests::defuse::DefuseExt;
use crate::tests::defuse::tokens::nep141::traits::DefuseFtReceiver;
use crate::{
    tests::defuse::env::Env,
    utils::{ft::FtExt, mt::MtExt, wnear::WNearExt},
};
use defuse::core::intents::tokens::FtWithdraw;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::{FeesConfig, Pips},
};
use defuse_test_utils::asserts::ResultAssertsExt;
use near_sdk::{AccountId, Gas, NearToken};
use rstest::rstest;

#[tokio::test]
#[rstest]
#[trace]
async fn ft_withdraw_intent() {
    use crate::tests::defuse::DefuseSignerExt;

    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    {
        env.defuse_ft_deposit_to(&ft, 1000, user.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), user.id(), &token_id.to_string())
                .await
                .unwrap(),
            1000
        );
    }

    let initial_withdraw_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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

    env.defuse
        .execute_intents(env.defuse.id(), [initial_withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &token_id.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.ft_token_balance_of(&ft, &other_user_id).await.unwrap(),
        0
    );

    let missing_storage_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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

    env.defuse
        .execute_intents(env.defuse.id(), [missing_storage_payload])
        .await
        .unwrap_err();

    // send user some near
    env.transfer_near(user.id(), STORAGE_DEPOSIT)
        .await
        .unwrap()
        .into_result()
        .unwrap();

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

    let old_defuse_balance = env
        .defuse
        .as_account()
        .view_account()
        .await
        .unwrap()
        .balance;

    // too large min_gas specified
    let too_large_min_gas_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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

    env.defuse_execute_intents(env.defuse.id(), [too_large_min_gas_payload])
        .await
        .assert_err_contains("Exceeded the prepaid gas.");

    let valid_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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

    env.defuse_execute_intents(env.defuse.id(), [valid_payload])
        .await
        .unwrap();

    let new_defuse_balance = env
        .defuse
        .as_account()
        .view_account()
        .await
        .unwrap()
        .balance;

    assert!(
        new_defuse_balance >= old_defuse_balance,
        "contract balance must not decrease"
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    // The storage deposit consumed the wNEAR balance
    assert_eq!(
        env.mt_contract_balance_of(
            env.defuse.id(),
            user.id(),
            &TokenId::from(Nep141TokenId::new(env.wnear.id().clone())).to_string()
        )
        .await
        .unwrap(),
        0,
    );

    assert_eq!(
        env.ft_token_balance_of(&ft, &other_user_id).await.unwrap(),
        1000
    );
}

#[tokio::test]
#[rstest]
#[trace]
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

    env.initial_ft_storage_deposit(vec![user.id(), defuse2.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.clone()));

    // too small min_gas
    {
        let low_min_gas_payload = user
            .sign_defuse_payload_default(
                env.defuse.id(),
                [FtWithdraw {
                    token: ft.clone(),
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

        env.defuse
            .execute_intents(env.defuse.id(), [low_min_gas_payload])
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
                .await
                .unwrap(),
            600
        );
        assert_eq!(
            env.ft_token_balance_of(&ft, env.defuse.id()).await.unwrap(),
            600
        );

        assert_eq!(
            env.ft_token_balance_of(&ft, defuse2.id()).await.unwrap(),
            400
        );
        assert_eq!(
            env.mt_contract_balance_of(defuse2.id(), &other_user_id, &ft1.to_string())
                .await
                .unwrap(),
            400
        );
    }

    let remaining_withdraw_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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

    env.defuse
        .execute_intents(env.defuse.id(), [remaining_withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, env.defuse.id()).await.unwrap(),
        0
    );

    assert_eq!(
        env.ft_token_balance_of(&ft, defuse2.id()).await.unwrap(),
        1000
    );
    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), &other_user_id, &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
}
