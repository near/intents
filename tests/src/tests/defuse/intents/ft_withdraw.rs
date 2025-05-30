use super::ExecuteIntentsExt;
use crate::{
    tests::defuse::{
        DefuseExt, DefuseSigner, SigningStandard, env::Env, tokens::nep141::DefuseFtReceiver,
    },
    utils::{ft::FtExt, mt::MtExt, wnear::WNearExt},
};
use arbitrary::{Arbitrary, Unstructured};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::{
        Deadline,
        fees::{FeesConfig, Pips},
        intents::{DefuseIntents, tokens::FtWithdraw},
        tokens::TokenId,
    },
};
use near_sdk::{AccountId, NearToken};
use randomness::Rng;
use rstest::rstest;
use std::time::Duration;
use test_utils::random::make_seedable_rng;
use test_utils::random::{Seed, random_seed};

#[tokio::test]
#[rstest]
#[trace]
async fn ft_withdraw_intent(random_seed: Seed, #[values(false, true)] no_registration: bool) {
    // intentionally large deposit
    const STORAGE_DEPOSIT: NearToken = NearToken::from_near(1000);

    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    let ft1 = TokenId::Nep141(env.ft1.clone());
    {
        env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000
        );
    }

    let nonce = rng.random();

    let intents = [env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
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
    )];

    // Test events emitted from the simulation
    {
        let simulation_output = env.defuse.simulate_intents(intents.clone()).await.unwrap();
        // Expecting two events, one for withdrawal, and one for execution of the intent

        assert_eq!(simulation_output.emitted_events.len(), 2);
        {
            let withdraw_event = simulation_output
                .emitted_events
                .iter()
                .find(|v| v.get("event").unwrap() == "ft_withdraw")
                .unwrap();

            assert_eq!(withdraw_event.get("standard").unwrap(), "dip4");
            assert!(withdraw_event.get("version").is_some());

            let data = withdraw_event.get("data").unwrap().as_array().unwrap();
            assert_eq!(data.len(), 1);
            let data = data.first().unwrap();

            assert_eq!(data.get("account_id").unwrap(), &env.user1.id().to_string());
            assert_eq!(data.get("receiver_id").unwrap(), &other_user_id.to_string());
            assert_eq!(data.get("token").unwrap(), &env.ft1.to_string());
            assert_eq!(data.get("amount").unwrap(), "1000");
        }

        {
            let intent_exec_event = simulation_output
                .emitted_events
                .iter()
                .find(|v| v.get("event").unwrap() == "intents_executed")
                .unwrap();
            assert_eq!(intent_exec_event.get("standard").unwrap(), "dip4");
            assert!(intent_exec_event.get("version").is_some());

            let data = intent_exec_event.get("data").unwrap().as_array().unwrap();
            assert_eq!(data.len(), 1);
            let data = data.first().unwrap();

            assert_eq!(data.get("account_id").unwrap(), &env.user1.id().to_string());
        }
    }

    env.defuse.execute_intents(intents).await.unwrap();

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

    let nonce = rng.random();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
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

    if no_registration {
        // IN no_registration case, only token owner can register a new user
        env.poa_factory
            .ft_storage_deposit_many(&env.ft1, &[&other_user_id])
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

    env.defuse_execute_intents(
        env.defuse.id(),
        [env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: other_user_id.clone(),
                    amount: 1000.into(),
                    memo: None,
                    msg: None,

                    storage_deposit,
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

    if !no_registration {
        // When no_registration is enabled, the storage deposit is done manually, not through intents
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
    }

    assert_eq!(
        env.ft_token_balance_of(&env.ft1, &other_user_id)
            .await
            .unwrap(),
        1000
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn ft_withdraw_intent_msg(random_seed: Seed, #[values(false, true)] no_registration: bool) {
    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

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

    env.poa_factory
        .ft_storage_deposit_many(&env.ft1, &[defuse2.id()])
        .await
        .unwrap();

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let nonce = rng.random();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            nonce,
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
