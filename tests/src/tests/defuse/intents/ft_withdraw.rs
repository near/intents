use defuse_sandbox::{
    assert_eq_defuse_event_logs,
    extensions::{
        DEFAULT_GAS,
        defuse::{
            DefuseDeployerExt, DefuseExt, DefuseSignerExt, ToEventLog,
            contract::config::{DefuseConfig, RolesConfig},
            core::{
                fees::{FeesConfig, Pips},
                intents::tokens::FtWithdraw,
                token_id::{TokenId, nep141::Nep141TokenId},
            },
        },
        mt::{Mt, MtBalanceOfArgs},
        wnear::WNearExt,
    },
    kit::{AccountId, Final, Gas, NearToken},
};
use defuse_test_utils::wasms::DEFUSE_WASM;
use rstest::rstest;

use crate::{
    tests::defuse::env::{Env, env},
    utils::asserts::ResultAssertsExt,
};

#[rstest]
#[tokio::test]
async fn ft_withdraw_intent(#[future(awt)] env: Env) {
    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.contract_id().clone()));

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    {
        env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
            .await
            .unwrap();

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            1000
        );
    }

    let initial_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
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

    let (res, _) = env
        .defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [initial_withdraw_payload.clone()],
        )
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(initial_withdraw_payload.to_event_log(), res.logs());

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &token_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        1000
    );

    assert_eq!(ft.balance_of(&other_user_id).await.unwrap().raw(), 0);

    let missing_storage_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
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

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [missing_storage_payload])
        .await
        .unwrap_err();

    // send user some near
    env.transaction(user.account_id())
        .transfer(STORAGE_DEPOSIT)
        .await
        .unwrap();

    // wrap NEAR
    user.near_deposit(env.wnear.contract_id(), STORAGE_DEPOSIT)
        .await
        .unwrap();

    // deposit wNEAR
    user.ft(env.wnear.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            STORAGE_DEPOSIT.as_yoctonear(),
            String::new(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    let old_defuse_balance = env.account(env.defuse.contract_id()).await.unwrap().amount;

    // too large min_gas specified
    let too_large_min_gas_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
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

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [too_large_min_gas_payload])
        .await
        .assert_err_contains("Exceeded the prepaid gas.");

    let valid_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
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

    let (res, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [valid_payload.clone()])
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(valid_payload.to_event_log(), res.logs());

    let new_defuse_balance = env.account(env.defuse.contract_id()).await.unwrap().amount;

    assert!(
        new_defuse_balance >= old_defuse_balance,
        "contract balance must not decrease"
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &token_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );

    // The storage deposit consumed the wNEAR balance
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &TokenId::from(Nep141TokenId::new(env.wnear.contract_id().clone()))
                    .to_string(),
            })
            .await
            .unwrap()
            .0,
        0,
    );

    assert_eq!(ft.balance_of(&other_user_id).await.unwrap().raw(), 1000);
}

#[rstest]
#[tokio::test]
async fn ft_withdraw_intent_msg(#[future(awt)] env: Env) {
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

    // too small min_gas
    {
        let low_min_gas_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [FtWithdraw {
                    token: ft.contract_id().clone(),
                    receiver_id: defuse2.account_id().clone(),
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

        let (res, _) = env
            .defuse_simulate_and_execute_intents(
                env.defuse.contract_id(),
                [low_min_gas_payload.clone()],
            )
            .await
            .unwrap();

        assert_eq_defuse_event_logs!(low_min_gas_payload.to_event_log(), res.logs());

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft1.to_string(),
                })
                .await
                .unwrap()
                .0,
            600
        );

        assert_eq!(
            ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
            600
        );

        assert_eq!(
            ft.balance_of(defuse2.account_id()).await.unwrap().raw(),
            400
        );
        assert_eq!(
            env.contract::<Mt>(defuse2.account_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &other_user_id,
                    token_id: &ft1.to_string(),
                })
                .await
                .unwrap()
                .0,
            400
        );
    }

    let remaining_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
                receiver_id: defuse2.account_id().clone(),
                amount: 600.into(),
                memo: Some("defuse-to-defuse".to_string()),
                msg: Some(other_user_id.to_string()),
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    let (res, _) = env
        .defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [remaining_withdraw_payload.clone()],
        )
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(remaining_withdraw_payload.to_event_log(), res.logs());

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
        ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
        0
    );

    assert_eq!(
        ft.balance_of(defuse2.account_id()).await.unwrap().raw(),
        1000
    );
    assert_eq!(
        env.contract::<Mt>(defuse2.account_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: &other_user_id,
                token_id: &ft1.to_string(),
            })
            .await
            .unwrap()
            .0,
        1000
    );
}
