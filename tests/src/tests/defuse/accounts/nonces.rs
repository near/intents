use arbitrary::{Arbitrary, Unstructured};
use chrono::{TimeDelta, Utc};
use defuse::core::{Deadline, ExpirableNonce, intents::DefuseIntents};
use itertools::Itertools;

use std::time::Duration;
use tokio::time::sleep;

use defuse_test_utils::{
    asserts::ResultAssertsExt,
    random::{Rng, rng},
};
use near_sdk::AccountId;
use rstest::rstest;

use crate::tests::defuse::{
    DefuseSigner, SigningStandard, accounts::AccountManagerExt, env::Env,
    intents::ExecuteIntentsExt,
};

#[tokio::test]
#[rstest]
async fn test_commit_nonces(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().build().await;
    let current_timestamp = Utc::now();
    let timeout_delta = TimeDelta::seconds(4);

    // legacy nonce
    let deadline = Deadline::MAX;
    let legacy_nonce = rng.random();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            legacy_nonce,
            deadline,
            DefuseIntents { intents: [].into() },
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
    let deadline = Deadline::new(current_timestamp.checked_sub_signed(timeout_delta).unwrap());
    let expired_nonce = ExpirableNonce::new(deadline, rng.random::<[u8; 20]>()).into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expired_nonce,
            deadline,
            DefuseIntents { intents: [].into() },
        )])
        .await
        .assert_err_contains("deadline has expired");

    // deadline is greater than nonce
    let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
    let expired_nonce = ExpirableNonce::new(deadline, rng.random::<[u8; 20]>()).into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expired_nonce,
            Deadline::MAX,
            DefuseIntents { intents: [].into() },
        )])
        .await
        .assert_err_contains("deadline is greater than nonce");

    // nonce can be committed
    let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
    let expirable_nonce = ExpirableNonce::new(deadline, rng.random::<[u8; 20]>()).into();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expirable_nonce,
            deadline,
            DefuseIntents { intents: [].into() },
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
async fn test_cleanup_expired_nonces(#[notrace] mut rng: impl Rng) {
    const WAITING_TIME: TimeDelta = TimeDelta::seconds(3);

    let env = Env::builder().build().await;
    let current_timestamp = Utc::now();

    // commit expirable nonces
    let deadline = Deadline::new(
        current_timestamp
            .checked_add_signed(TimeDelta::seconds(1))
            .unwrap(),
    );
    let expirable_nonce = ExpirableNonce::new(deadline, rng.random::<[u8; 20]>()).into();

    let long_term_deadline = Deadline::new(
        current_timestamp
            .checked_add_signed(TimeDelta::hours(1))
            .unwrap(),
    );
    let long_term_expirable_nonce =
        ExpirableNonce::new(long_term_deadline, rng.random::<[u8; 20]>()).into();

    env.defuse
        .execute_intents([
            env.user1.sign_defuse_message(
                SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                    .unwrap(),
                env.defuse.id(),
                expirable_nonce,
                deadline,
                DefuseIntents { intents: [].into() },
            ),
            env.user1.sign_defuse_message(
                SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                    .unwrap(),
                env.defuse.id(),
                long_term_expirable_nonce,
                long_term_deadline,
                DefuseIntents { intents: [].into() },
            ),
        ])
        .await
        .unwrap();

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &expirable_nonce)
            .await
            .unwrap(),
    );

    sleep(Duration::from_secs_f64(WAITING_TIME.as_seconds_f64())).await;

    // nonce is expired
    env.defuse
        .cleanup_expired_nonces(&[(env.user1.id().clone(), vec![expirable_nonce])])
        .await
        .unwrap();

    assert!(
        !env.defuse
            .is_nonce_used(env.user1.id(), &expirable_nonce)
            .await
            .unwrap(),
    );

    let unknown_user: AccountId = "unknown-user.near".parse().unwrap();

    // skip if nonce already cleared / is not expired / user does not exist
    env.defuse
        .cleanup_expired_nonces(&[
            (env.user1.id().clone(), vec![expirable_nonce]),
            (env.user1.id().clone(), vec![long_term_expirable_nonce]),
            (unknown_user, vec![expirable_nonce]),
        ])
        .await
        .unwrap();
}

#[tokio::test]
#[rstest]
async fn cleanup_multiple_nonces(
    #[notrace] mut rng: impl Rng,
    #[values(1, 10, 100)] nonce_count: usize,
) {
    const CHUNK_SIZE: usize = 10;
    const WAITING_TIME: TimeDelta = TimeDelta::seconds(3);

    let env = Env::builder().build().await;
    let mut nonces = Vec::with_capacity(nonce_count);

    for chunk in &(0..nonce_count).chunks(CHUNK_SIZE) {
        let current_timestamp = Utc::now();

        let intents = chunk
            .map(|_| {
                // commit expirable nonce
                let deadline =
                    Deadline::new(current_timestamp.checked_add_signed(WAITING_TIME).unwrap());
                let expirable_nonce =
                    ExpirableNonce::new(deadline, rng.random::<[u8; 20]>()).into();

                nonces.push(expirable_nonce);

                env.user1.sign_defuse_message(
                    SigningStandard::Nep413,
                    env.defuse.id(),
                    expirable_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
            })
            .collect::<Vec<_>>();

        env.defuse.execute_intents(intents).await.unwrap();
    }

    sleep(Duration::from_secs_f64(WAITING_TIME.as_seconds_f64())).await;

    let gas_used = env
        .defuse
        .cleanup_expired_nonces(&[(env.user1.id().clone(), nonces)])
        .await
        .unwrap();

    println!(
        "Gas used to clear {} nonces: {}",
        nonce_count,
        gas_used.total_gas_burnt(),
    );
}
