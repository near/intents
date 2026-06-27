use std::collections::BTreeSet;

use crate::{
    tests::defuse::env::{Env, env},
    utils::asserts::ResultAssertsExt,
};
use defuse_sandbox::extensions::{
    acl::AccessControllableExt,
    defuse::{
        DefuseExt, SaltArgs,
        contract::Role,
        core::{accounts::SaltRotationEvent, events::DefuseEvent},
    },
};
use near_sdk_core::events::AsNep297Event;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn update_current_salt(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (prev_salt, user1, user2) = futures::join!(
        env.defuse.current_salt().into_future(),
        env.create_user(),
        env.create_user()
    );
    let prev_salt = prev_salt.unwrap();

    // only DAO or salt manager can rotate salt
    {
        user2
            .defuse_update_current_salt(env.defuse.contract_id().clone())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // rotate salt by salt manager
    {
        env.acl_grant_role(
            env.defuse.contract_id().clone(),
            Role::SaltManager,
            user1.account_id().clone(),
        )
        .await
        .expect("failed to grant role");

        let (res, new_salt) = user1
            .defuse_update_current_salt(env.defuse.contract_id().clone())
            .await
            .expect("unable to rotate salt");

        let event = DefuseEvent::SaltRotation(SaltRotationEvent {
            invalidated: BTreeSet::new(),
            current: new_salt,
        })
        .to_nep297_event()
        .to_event_log();

        assert!(res.logs().contains(&event));

        let (current_salt, prev_salt_is_valid) = futures::join!(
            env.defuse.current_salt().into_future(),
            env.defuse
                .is_valid_salt(SaltArgs { salt: prev_salt })
                .into_future()
        );
        let current_salt = current_salt.unwrap();

        assert_ne!(prev_salt, current_salt);
        assert_eq!(new_salt, current_salt);
        assert!(prev_salt_is_valid.unwrap());
    }
}

#[rstest]
#[tokio::test]
async fn invalidate_salts(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (current_salt, user1, user2) = futures::join!(
        env.defuse.current_salt().into_future(),
        env.create_user(),
        env.create_user()
    );
    let mut current_salt = current_salt.unwrap();
    let mut prev_salt = current_salt;

    // only DAO or salt manager can invalidate salt
    {
        user2
            .defuse_invalidate_salts(env.defuse.contract_id().clone(), [prev_salt])
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // invalidate prev salt by salt manager
    {
        env.acl_grant_role(
            env.defuse.contract_id().clone(),
            Role::SaltManager,
            user1.account_id().clone(),
        )
        .await
        .expect("failed to grant role");

        (_, current_salt) = user1
            .defuse_update_current_salt(env.defuse.contract_id().clone())
            .await
            .expect("unable to rotate salt");

        let (res, current_salt) = user1
            .defuse_invalidate_salts(env.defuse.contract_id().clone(), [prev_salt])
            .await
            .expect("unable to rotate salt");

        let event = DefuseEvent::SaltRotation(SaltRotationEvent {
            invalidated: std::iter::once(prev_salt).collect(),
            current: current_salt,
        })
        .to_nep297_event()
        .to_event_log();

        assert!(res.logs().contains(&event));

        assert!(
            !env.defuse
                .is_valid_salt(SaltArgs { salt: prev_salt })
                .await
                .unwrap()
        );
    }

    // invalidate current salt by salt manager
    {
        prev_salt = current_salt;
        let (res, current_salt) = user1
            .defuse_invalidate_salts(env.defuse.contract_id().clone(), [current_salt])
            .await
            .expect("unable to rotate salt");

        let event = DefuseEvent::SaltRotation(SaltRotationEvent {
            invalidated: std::iter::once(prev_salt).collect(),
            current: current_salt,
        })
        .to_nep297_event()
        .to_event_log();

        assert!(res.logs().contains(&event));

        assert!(
            !env.defuse
                .is_valid_salt(SaltArgs { salt: prev_salt })
                .await
                .unwrap()
        );
        assert_ne!(prev_salt, current_salt);
    }
}
