use arbitrary::{Arbitrary, Unstructured};
use chrono::{TimeDelta, Utc};
use defuse::{
    contract::Role,
    core::{Deadline, Nonce, Salt, intents::DefuseIntents},
};
use itertools::Itertools;

use std::time::Duration;
use tokio::time::sleep;

use defuse_test_utils::{
    asserts::ResultAssertsExt,
    random::{Rng, random_bytes, rng},
};
use near_sdk::AccountId;
use rstest::rstest;

use crate::{
    tests::defuse::{
        accounts::AccountManagerExt, env::{create_random_salted_nonce, Env}, garbage_collector::GarbageCollectorExt, intents::ExecuteIntentsExt, state::SaltManagerExt, DefusePayloadBuilder, DefuseSigner, SigningStandard
    },
    utils::acl::AclExt,
};

use futures::future::try_join_all;

#[tokio::test]
#[rstest]
async fn test_commit_nonces(random_bytes: Vec<u8>, #[notrace] mut rng: impl Rng) {
    let env = Env::builder().deployer_as_super_admin().build().await;
    let current_timestamp = Utc::now();
    let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();
    let timeout_delta = TimeDelta::days(1);
    let u = &mut Unstructured::new(&random_bytes);

    let user = env.create_user().await;

    // legacy nonce
    {
        let deadline = Deadline::MAX;
        let legacy_nonce = rng.random();

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    legacy_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(user.id(), &legacy_nonce)
                .await
                .unwrap(),
        );
    }

    // invalid salt
    {
        let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
        let random_salt = Salt::arbitrary(u).unwrap();
        let salted = create_random_salted_nonce(random_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    salted,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .assert_err_contains("invalid salt");
    }

    // deadline is greater than nonce
    {
        let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
        let expired_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    expired_nonce,
                    Deadline::MAX,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .assert_err_contains("deadline is greater than nonce");
    }

    // nonce is expired
    {
        let deadline = Deadline::new(current_timestamp.checked_sub_signed(timeout_delta).unwrap());
        let expired_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    expired_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .assert_err_contains("deadline has expired");
    }

    // nonce can be committed
    {
        let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
        let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    expirable_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(user.id(), &expirable_nonce)
                .await
                .unwrap(),
        );
    }

    // nonce can be committed with previous salt
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user.id())
            .await
            .expect("failed to grant role");

        user.update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
        let old_salt_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    old_salt_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(user.id(), &old_salt_nonce)
                .await
                .unwrap(),
        );
    }

    // nonce can't be committed with invalidated salt
    {
        let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();
        user.invalidate_salts(env.defuse.id(), &[current_salt])
            .await
            .expect("unable to invalidate salt");

        let deadline = Deadline::new(current_timestamp.checked_add_signed(timeout_delta).unwrap());
        let invalid_salt_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse
            .execute_intents(
                env.defuse.id(),
                [user.sign_defuse_message(
                    SigningStandard::arbitrary(u).unwrap(),
                    env.defuse.id(),
                    invalid_salt_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )],
            )
            .await
            .assert_err_contains("invalid salt");
    }
}

#[tokio::test]
#[rstest]
async fn test_cleanup_nonces(#[notrace] mut rng: impl Rng) {
    const WAITING_TIME: TimeDelta = TimeDelta::seconds(3);

    let env = Env::builder().deployer_as_super_admin().build().await;
    let user = env.create_user().await;

    let current_timestamp = Utc::now();
    let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

    let deadline = Deadline::new(
        current_timestamp
            .checked_add_signed(TimeDelta::seconds(1))
            .unwrap(),
    );

    let long_term_deadline = Deadline::new(
        current_timestamp
            .checked_add_signed(TimeDelta::hours(1))
            .unwrap(),
    );

    let legacy_nonce: Nonce = rng.random();
    let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);
    let long_term_expirable_nonce =
        create_random_salted_nonce(current_salt, long_term_deadline, &mut rng);

    // commit nonces
    {
        env.defuse
            .execute_intents(
                env.defuse.id(),
                [
                    user.sign_defuse_message(
                        SigningStandard::arbitrary(&mut Unstructured::new(
                            &rng.random::<[u8; 1]>(),
                        ))
                        .unwrap(),
                        env.defuse.id(),
                        legacy_nonce,
                        deadline,
                        DefuseIntents { intents: [].into() },
                    ),
                    user.sign_defuse_message(
                        SigningStandard::arbitrary(&mut Unstructured::new(
                            &rng.random::<[u8; 1]>(),
                        ))
                        .unwrap(),
                        env.defuse.id(),
                        expirable_nonce,
                        deadline,
                        DefuseIntents { intents: [].into() },
                    ),
                    user.sign_defuse_message(
                        SigningStandard::arbitrary(&mut Unstructured::new(
                            &rng.random::<[u8; 1]>(),
                        ))
                        .unwrap(),
                        env.defuse.id(),
                        long_term_expirable_nonce,
                        long_term_deadline,
                        DefuseIntents { intents: [].into() },
                    ),
                ],
            )
            .await
            .unwrap();
    }

    sleep(Duration::from_secs_f64(WAITING_TIME.as_seconds_f64())).await;

    // only DAO or garbage collector can cleanup nonces
    {
        user.cleanup_nonces(
            env.defuse.id(),
            vec![(user.id().clone(), vec![expirable_nonce])],
        )
        .await
        .assert_err_contains("Insufficient permissions for method");
    }

    // nonce is expired
    {
        env.acl_grant_role(env.defuse.id(), Role::GarbageCollector, user.id())
            .await
            .expect("failed to grant role");

        user.cleanup_nonces(
            env.defuse.id(),
            vec![(user.id().clone(), vec![expirable_nonce])],
        )
        .await
        .unwrap();

        assert!(
            !env.defuse
                .is_nonce_used(user.id(), &expirable_nonce)
                .await
                .unwrap(),
        );
    }

    // skip if nonce is legacy / already cleared / is not expired / user does not exist
    {
        let unknown_user: AccountId = "unknown-user.near".parse().unwrap();

        user.cleanup_nonces(
            env.defuse.id(),
            vec![
                (user.id().clone(), vec![expirable_nonce]),
                (user.id().clone(), vec![legacy_nonce]),
                (user.id().clone(), vec![long_term_expirable_nonce]),
                (unknown_user, vec![expirable_nonce]),
            ],
        )
        .await
        .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(user.id(), &legacy_nonce)
                .await
                .unwrap(),
        );

        assert!(
            env.defuse
                .is_nonce_used(user.id(), &long_term_expirable_nonce)
                .await
                .unwrap(),
        );
    }

    // clean invalid salt
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user.id())
            .await
            .expect("failed to grant role");

        user.invalidate_salts(env.defuse.id(), &[current_salt])
            .await
            .expect("unable to rotate salt");

        user.cleanup_nonces(
            env.defuse.id(),
            vec![(user.id().clone(), vec![long_term_expirable_nonce])],
        )
        .await
        .unwrap();

        assert!(
            !env.defuse
                .is_nonce_used(user.id(), &long_term_expirable_nonce)
                .await
                .unwrap(),
        );
    }
}

#[tokio::test]
#[rstest]
async fn cleanup_multiple_nonces(
    #[notrace] mut rng: impl Rng,
    #[values(1, 10, 100)] nonce_count: usize,
) {
    const CHUNK_SIZE: usize = 10;
    const WAITING_TIME: TimeDelta = TimeDelta::seconds(3);

    let env = Env::builder().deployer_as_super_admin().build().await;
    let user = env.create_user().await;

    let mut nonces = Vec::with_capacity(nonce_count);
    let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

    env.acl_grant_role(env.defuse.id(), Role::GarbageCollector, user.id())
        .await
        .expect("failed to grant role");

    for chunk in &(0..nonce_count).chunks(CHUNK_SIZE) {
        let current_timestamp = Utc::now();

        let intents = chunk
            .map(|_| {
                // commit expirable nonce
                let deadline =
                    Deadline::new(current_timestamp.checked_add_signed(WAITING_TIME).unwrap());
                let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

                nonces.push(expirable_nonce);

                user.sign_defuse_message(
                    SigningStandard::Nep413,
                    env.defuse.id(),
                    expirable_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
            })
            .collect::<Vec<_>>();

        env.defuse
            .execute_intents(env.defuse.id(), intents)
            .await
            .unwrap();
    }

    sleep(Duration::from_secs_f64(WAITING_TIME.as_seconds_f64())).await;

    let gas_used = user
        .cleanup_nonces(env.defuse.id(), vec![(user.id().clone(), nonces.clone())])
        .await
        .unwrap();

    let checks = try_join_all(
        nonces
            .iter()
            .map(|n| env.defuse.is_nonce_used(user.id(), n)),
    )
    .await
    .expect("Failed to check cleaned nonces");

    assert!(checks.into_iter().all(|is_used| !is_used));

    println!(
        "Gas used to clear {} nonces: {}",
        nonce_count,
        gas_used.total_gas_burnt(),
    );
}
