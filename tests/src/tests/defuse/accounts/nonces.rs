use arbitrary::{Arbitrary, Unstructured};
use defuse_sandbox::{
    extensions::{
        acl::AccessControllableExt,
        defuse::{
            DefuseExt, DefuseSignerExt, IsNonceUsedArgs,
            contract::Role,
            core::{Nonce, Salt, Timestamp, intents::DefuseIntents},
            create_random_salted_nonce,
        },
    },
    kit::AccountId,
};
use futures::future::join_all;
use rstest::rstest;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    tests::defuse::env::{Env, env},
    utils::{
        asserts::ResultAssertsExt,
        random::{Rng, RngExt, random_bytes, rng},
    },
};

#[rstest]
#[tokio::test]
async fn test_commit_nonces(
    random_bytes: Vec<u8>,
    #[notrace] mut rng: impl Rng,
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let current_timestamp = Timestamp::now();
    let current_salt = env.defuse.current_salt().await.unwrap();
    let timeout_delta = Duration::from_hours(24);
    let u = &mut Unstructured::new(&random_bytes);

    let user = env.create_user().await;

    // legacy nonce
    {
        let deadline = Timestamp::MAX;
        let legacy_nonce = rng.random();

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    legacy_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &legacy_nonce,
                })
                .await
                .unwrap(),
        );
    }

    // invalid salt
    {
        let deadline = current_timestamp + timeout_delta;
        let random_salt = Salt::arbitrary(u).unwrap();
        let salted = create_random_salted_nonce(random_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    salted,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .assert_err_contains("invalid salt");
    }

    // deadline is greater than nonce
    {
        let deadline = current_timestamp + timeout_delta;
        let expired_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    expired_nonce,
                    Timestamp::MAX,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .assert_err_contains("deadline is greater than nonce");
    }

    // nonce is expired
    {
        let deadline = current_timestamp - timeout_delta;
        let expired_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    expired_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .assert_err_contains("deadline has expired");
    }

    // nonce can be committed
    {
        let deadline = current_timestamp + timeout_delta;
        let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    expirable_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &expirable_nonce,
                })
                .await
                .unwrap(),
        );
    }

    // nonce can be committed with previous salt
    {
        env.acl_grant_role(
            env.defuse.contract_id(),
            Role::SaltManager,
            user.account_id(),
        )
        .await
        .expect("failed to grant role");

        user.defuse_update_current_salt(env.defuse.contract_id())
            .await
            .expect("unable to rotate salt");

        let deadline = current_timestamp + timeout_delta;
        let old_salt_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    old_salt_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &old_salt_nonce,
                })
                .await
                .unwrap(),
        );
    }

    // nonce can't be committed with invalidated salt
    {
        let current_salt = env.defuse.current_salt().await.unwrap();
        user.defuse_invalidate_salts(env.defuse.contract_id(), [current_salt])
            .await
            .expect("unable to invalidate salt");

        let deadline = current_timestamp + timeout_delta;
        let invalid_salt_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [user
                .sign_defuse_message(
                    env.defuse.contract_id(),
                    invalid_salt_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                )
                .await],
        )
        .await
        .assert_err_contains("invalid salt");
    }
}

#[rstest]
#[tokio::test]
async fn test_cleanup_nonces(
    #[notrace] mut rng: impl Rng,
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    const WAITING_TIME: Duration = Duration::from_secs(3);
    let user = env.create_user().await;

    let current_timestamp = Timestamp::now();
    let current_salt = env.defuse.current_salt().await.unwrap();

    let deadline = current_timestamp + Duration::from_secs(1);
    let long_term_deadline = current_timestamp + Duration::from_hours(1);

    let legacy_nonce: Nonce = rng.random();
    let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);
    let long_term_expirable_nonce =
        create_random_salted_nonce(current_salt, long_term_deadline, &mut rng);

    // commit nonces
    {
        env.defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            join_all([
                user.sign_defuse_message(
                    env.defuse.contract_id(),
                    legacy_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                ),
                user.sign_defuse_message(
                    env.defuse.contract_id(),
                    expirable_nonce,
                    deadline,
                    DefuseIntents { intents: [].into() },
                ),
                user.sign_defuse_message(
                    env.defuse.contract_id(),
                    long_term_expirable_nonce,
                    long_term_deadline,
                    DefuseIntents { intents: [].into() },
                ),
            ])
            .await,
        )
        .await
        .unwrap();
    }

    sleep(WAITING_TIME).await;

    // only DAO or garbage collector can cleanup nonces
    {
        user.defuse_cleanup_nonces(
            env.defuse.contract_id(),
            vec![(user.account_id().clone(), vec![expirable_nonce])],
        )
        .await
        .assert_err_contains("Insufficient permissions for method");
    }

    // nonce is expired
    {
        env.acl_grant_role(
            env.defuse.contract_id(),
            Role::GarbageCollector,
            user.account_id(),
        )
        .await
        .expect("failed to grant role");

        user.defuse_cleanup_nonces(
            env.defuse.contract_id(),
            vec![(user.account_id().clone(), vec![expirable_nonce])],
        )
        .await
        .unwrap();

        assert!(
            !env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &expirable_nonce,
                })
                .await
                .unwrap(),
        );
    }

    // skip if nonce is legacy / already cleared / is not expired / user does not exist
    {
        let unknown_user: AccountId = "unknown-user.near".parse().unwrap();

        user.defuse_cleanup_nonces(
            env.defuse.contract_id(),
            vec![
                (user.account_id().clone(), vec![expirable_nonce]),
                (user.account_id().clone(), vec![legacy_nonce]),
                (user.account_id().clone(), vec![long_term_expirable_nonce]),
                (unknown_user, vec![expirable_nonce]),
            ],
        )
        .await
        .unwrap();

        assert!(
            env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &legacy_nonce,
                })
                .await
                .unwrap(),
        );

        assert!(
            env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &long_term_expirable_nonce,
                })
                .await
                .unwrap(),
        );
    }

    // clean invalid salt
    {
        env.acl_grant_role(
            env.defuse.contract_id(),
            Role::SaltManager,
            user.account_id(),
        )
        .await
        .expect("failed to grant role");

        user.defuse_invalidate_salts(env.defuse.contract_id(), [current_salt])
            .await
            .expect("unable to rotate salt");

        user.defuse_cleanup_nonces(
            env.defuse.contract_id(),
            vec![(user.account_id().clone(), vec![long_term_expirable_nonce])],
        )
        .await
        .unwrap();

        assert!(
            !env.defuse
                .is_nonce_used(IsNonceUsedArgs {
                    account_id: user.account_id(),
                    nonce: &long_term_expirable_nonce,
                })
                .await
                .unwrap(),
        );
    }
}

#[rstest]
#[tokio::test]
async fn cleanup_multiple_nonces(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
    #[notrace] mut rng: impl Rng,
    #[values(1, 10, 100)] nonce_count: usize,
) {
    use futures::StreamExt;

    const CHUNK_SIZE: usize = 10;
    const WAITING_TIME: Duration = Duration::from_secs(3);
    let user = env.create_user().await;

    let mut nonces = Vec::with_capacity(nonce_count);
    let current_salt = env.defuse.current_salt().await.unwrap();

    env.acl_grant_role(
        env.defuse.contract_id(),
        Role::GarbageCollector,
        user.account_id(),
    )
    .await
    .expect("failed to grant role");

    for start in (0..nonce_count).step_by(CHUNK_SIZE) {
        let end = (start + CHUNK_SIZE).min(nonce_count);
        let current_timestamp = Timestamp::now();

        let intents = join_all((start..end).map(|_| {
            // commit expirable nonce
            let deadline = current_timestamp + WAITING_TIME;
            let expirable_nonce = create_random_salted_nonce(current_salt, deadline, &mut rng);

            nonces.push(expirable_nonce);

            user.sign_defuse_message(
                env.defuse.contract_id(),
                expirable_nonce,
                deadline,
                DefuseIntents { intents: [].into() },
            )
        }))
        .await;

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), intents)
            .await
            .unwrap();
    }

    sleep(WAITING_TIME).await;

    user.defuse_cleanup_nonces(
        env.defuse.contract_id(),
        vec![(user.account_id().clone(), nonces.clone())],
    )
    .await
    .unwrap();

    assert!(
        futures::stream::iter(nonces)
            .all(|n| {
                let defuse = &env.defuse;
                let user_id = user.account_id().clone();

                async move {
                    !defuse
                        .is_nonce_used(IsNonceUsedArgs {
                            account_id: &user_id,
                            nonce: &n,
                        })
                        .await
                        .unwrap()
                }
            })
            .await
    );
}
