use super::ExecuteIntentsExt;
use crate::tests::defuse::DefuseExt;
use crate::tests::defuse::tokens::nep141::traits::DefuseFtReceiver;
use crate::{
    tests::defuse::{DefuseSigner, SigningStandard, env::Env},
    utils::{ft::FtExt, mt::MtExt, wnear::WNearExt},
};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::core::{
    Deadline,
    intents::{DefuseIntents, tokens::FtWithdraw},
};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::{FeesConfig, Pips},
};
use defuse_randomness::Rng;
use defuse_test_utils::{asserts::ResultAssertsExt, random::rng};
use near_sdk::{AccountId, Gas, NearToken};
use rstest::rstest;
use std::time::Duration;

#[tokio::test]
#[rstest]
#[trace]
async fn ft_withdraw_intent(
    #[notrace] mut rng: impl Rng,
    #[values(false, true)] no_registration: bool,
) {
    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let mut env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.get_or_create_user().await;
    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    let ft = env.create_token().await;
    let token_id = TokenId::from(Nep141TokenId::new(ft.clone()));
    env.ft_storage_deposit_for_users(vec![user.id()], &[&ft])
        .await;
    env.ft_deposit_to_root(&[&ft]).await;

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

    let nonce = rng.random();

    env.defuse
        .execute_intents([user.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: [FtWithdraw {
                    token: ft.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
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

    let nonce = rng.random();

    env.defuse
        .execute_intents([user.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: ft.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    // user has no wnear yet
                    storage_deposit: Some(STORAGE_DEPOSIT),
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
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

    if no_registration {
        // IN no_registration case, only token owner can register a new user
        env.poa_factory
            .ft_storage_deposit_many(&ft, &[&other_user_id])
            .await
            .unwrap();
    }

    // in case of registration enabled, the user now has wNEAR to pay for it
    let storage_deposit = (!no_registration).then_some(STORAGE_DEPOSIT);

    let old_defuse_balance = env
        .defuse
        .as_account()
        .view_account()
        .await
        .unwrap()
        .balance;

    let nonce = rng.random();

    // too large min_gas specified
    env.defuse_execute_intents(
        env.defuse.id(),
        [user.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: ft.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    storage_deposit,
                    min_gas: Some(Gas::from_tgas(300)),
                }
                .into()]
                .into(),
            },
        )],
    )
    .await
    .assert_err_contains("Exceeded the prepaid gas.");

    env.defuse_execute_intents(
        env.defuse.id(),
        [user.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: ft.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,
                    storage_deposit,
                    min_gas: None,
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
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    if !no_registration {
        // When no_registration is enabled, the storage deposit is done manually, not through intents
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
    }

    assert_eq!(
        env.ft_token_balance_of(&ft, &other_user_id).await.unwrap(),
        1000
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn ft_withdraw_intent_msg(
    #[notrace] mut rng: impl Rng,
    #[values(false, true)] no_registration: bool,
) {
    let mut env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.get_or_create_user().await;
    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    let ft = env.create_token().await;
    env.ft_storage_deposit_for_users(vec![user.id()], &[&ft])
        .await;
    env.ft_deposit_to_root(&[&ft]).await;

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

    env.poa_factory
        .ft_storage_deposit_many(&ft, &[defuse2.id()])
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.clone()));

    // too small min_gas
    {
        env.defuse
            .execute_intents([user.sign_defuse_message(
                SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                    .unwrap(),
                env.defuse.id(),
                rng.random(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [FtWithdraw {
                        token: ft.clone(),
                        receiver_id: defuse2.id().clone(),
                        amount: 400.into(),
                        memo: Some("defuse-to-defuse".to_string()),
                        msg: Some(other_user_id.to_string()),
                        storage_deposit: None,
                        // too small, but minimum of 30TGas will be used
                        min_gas: Some(Gas::from_tgas(1)),
                    }
                    .into()]
                    .into(),
                },
            )])
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

    env.defuse
        .execute_intents([user.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            rng.random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: [FtWithdraw {
                    token: ft.clone(),
                    receiver_id: defuse2.id().clone(),
                    amount: 600.into(),
                    memo: Some("defuse-to-defuse".to_string()),
                    msg: Some(other_user_id.to_string()),
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
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
