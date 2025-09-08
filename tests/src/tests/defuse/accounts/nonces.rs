use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{
    Deadline, ExpirableNonce,
    intents::{DefuseIntents, tokens::FtWithdraw},
    token_id::{TokenId, nep141::Nep141TokenId},
};

use std::time::Duration;
use tokio::time::sleep;

use defuse_test_utils::{
    asserts::ResultAssertsExt,
    random::{Rng, rng},
};
use near_sdk::json_types::U128;
use rstest::rstest;

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard, accounts::AccountManagerExt, env::Env,
        intents::ExecuteIntentsExt,
    },
    utils::mt::MtExt,
};

#[tokio::test]
#[rstest]
async fn test_commit_nonces(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().build().await;
    let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;

    let withdraw_amount: U128 = 1000.into();
    let deposit_amount = withdraw_amount.0 * 2;

    // Create user account

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    {
        env.defuse_ft_deposit_to(&env.ft1, deposit_amount, env.user1.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            deposit_amount
        );
    }

    // legacy nonce
    let legacy_nonce = rng.random();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            legacy_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
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

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &legacy_nonce)
            .await
            .unwrap(),
    );

    // nonce is expired
    let expired_nonce =
        ExpirableNonce::pack_expirable(current_timestamp - 10000, &rng.random::<[u8; 20]>())
            .unwrap()
            .into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expired_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
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
        .assert_err_contains("nonce was already expired");

    // nonce can be committed
    let expirable_nonce =
        ExpirableNonce::pack_expirable(current_timestamp + 10000, &rng.random::<[u8; 20]>())
            .unwrap()
            .into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expirable_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
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

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &expirable_nonce)
            .await
            .unwrap(),
    );
}

#[tokio::test]
#[rstest]
async fn test_clear_expired_nonces(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().build().await;
    let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;

    let withdraw_amount: U128 = 1000.into();
    let deposit_amount = withdraw_amount.0;

    // Create user account

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    {
        env.defuse_ft_deposit_to(&env.ft1, deposit_amount, env.user1.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            deposit_amount
        );
    }

    // commit expirable nonce
    let expirable_nonce =
        ExpirableNonce::pack_expirable(current_timestamp + 3000, &rng.random::<[u8; 20]>())
            .unwrap()
            .into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expirable_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
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

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &expirable_nonce)
            .await
            .unwrap(),
    );

    // nonce is still active
    env.defuse
        .clear_expired_nonces(&[(env.user1.id().clone(), vec![expirable_nonce])])
        .await
        .assert_err_contains("nonce is still active");

    sleep(Duration::from_secs(1)).await;

    // nonce is expired
    env.defuse
        .clear_expired_nonces(&[(env.user1.id().clone(), vec![expirable_nonce])])
        .await
        .unwrap();

    // skip if already cleared
    env.defuse
        .clear_expired_nonces(&[(env.user1.id().clone(), vec![expirable_nonce])])
        .await
        .unwrap();
}

#[tokio::test]
#[rstest]
async fn clear_multiple_nonces(
    #[notrace] mut rng: impl Rng,
    #[values(1, 10, 100)] nonce_count: usize,
) {
    let env = Env::builder().build().await;

    let deposit_amount = 1000;
    let withdraw_amount: U128 = (1000 / nonce_count as u128).into();
    let chunk_size = 10;

    // Create user account

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    {
        env.defuse_ft_deposit_to(&env.ft1, deposit_amount, env.user1.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            deposit_amount
        );
    }

    let mut nonces = Vec::with_capacity(nonce_count as usize);
    let rounds = (nonce_count + chunk_size - 1) / chunk_size;
    let balance_before = env.near_balance(env.id()).await;

    for _ in 0..rounds {
        let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;

        let intents = (0..chunk_size.min(nonce_count))
            .map(|_| {
                // commit expirable nonce
                let expirable_nonce = ExpirableNonce::pack_expirable(
                    current_timestamp + 3000,
                    &rng.random::<[u8; 20]>(),
                )
                .unwrap()
                .into();

                nonces.push(expirable_nonce);

                env.user1.sign_defuse_message(
                    SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                        .unwrap(),
                    env.defuse.id(),
                    expirable_nonce,
                    Deadline::MAX,
                    DefuseIntents {
                        intents: [FtWithdraw {
                            token: env.ft1.clone(),
                            receiver_id: env.user1.id().clone(),
                            amount: withdraw_amount,
                            memo: None,
                            msg: None,
                            storage_deposit: None,
                            min_gas: None,
                        }
                        .into()]
                        .into(),
                    },
                )
            })
            .collect::<Vec<_>>();

        env.defuse.execute_intents(intents).await.unwrap();
    }

    let balance_after = env.near_balance(env.id()).await;

    sleep(Duration::from_secs(rounds as u64 + 1)).await;

    let gas_used = env
        .defuse
        .clear_expired_nonces(&[(env.user1.id().clone(), nonces)])
        .await
        .unwrap();

    println!(
        "Gas used to clear {} nonces: {}, balance diff: {}",
        nonce_count,
        gas_used.total_gas_burnt(),
        balance_after.saturating_sub(balance_before)
    );
}
