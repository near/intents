use std::time::Duration;

use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::{
        fees::{FeesConfig, Pips},
        intents::{tokens::FtWithdraw, DefuseIntents},
        tokens::TokenId,
        Deadline,
    },
};
use near_sdk::{AccountId, NearToken};
use randomness::{make_true_rng, Rng};

use super::ExecuteIntentsExt;
use crate::{
    tests::defuse::{env::Env, tokens::nep141::DefuseFtReceiver, DefuseExt, DefuseSigner},
    utils::{ft::FtExt, mt::MtExt, wnear::WNearExt},
};

#[tokio::test]
async fn test_ft_withdraw_intent() {
    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let env = Env::new().await;

    env.defuse_ft_mint(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap();

    let ft1 = TokenId::Nep141(env.ft1.clone());
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.ft_token_balance_of(&env.ft1, &other_user_id)
            .await
            .unwrap(),
        0
    );

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    // user has no wnear yet
                    storage_deposit: Some(STORAGE_DEPOSIT),
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap_err();

    // send user some near
    env.transfer_near(env.user1.id(), STORAGE_DEPOSIT)
        .await
        .unwrap()
        .into_result()
        .unwrap();
    // wrap NEAR
    env.user1
        .near_deposit(env.wnear.id(), STORAGE_DEPOSIT)
        .await
        .unwrap();
    // deposit wNEAR
    env.user1
        .defuse_ft_deposit(
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
    env.defuse_execute_intents(
        env.defuse.id(),
        [env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    // now user has wNEAR to pay for it
                    storage_deposit: Some(STORAGE_DEPOSIT),
                }
                .into()]
                .into(),
            },
        )],
    )
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
        env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.mt_contract_balance_of(
            env.defuse.id(),
            env.user1.id(),
            &TokenId::Nep141(env.wnear.id().clone()).to_string()
        )
        .await
        .unwrap(),
        0,
    );

    assert_eq!(
        env.ft_token_balance_of(&env.ft1, &other_user_id)
            .await
            .unwrap(),
        1000
    );
}

#[tokio::test]
async fn test_ft_withdraw_intent_msg() {
    let env = Env::new().await;

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
        )
        .await
        .unwrap();
    env.ft_storage_deposit(&env.ft1, &[defuse2.id()])
        .await
        .unwrap();

    env.defuse_ft_mint(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: defuse2.id().clone(),
                    amount: 1000.into(),
                    memo: Some("defuse-to-defuse".to_string()),
                    msg: Some(env.user2.id().to_string()),
                    storage_deposit: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap();

    let ft1 = TokenId::Nep141(env.ft1.clone());

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&env.ft1, env.defuse.id())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.ft_token_balance_of(&env.ft1, defuse2.id())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), env.user2.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
}
