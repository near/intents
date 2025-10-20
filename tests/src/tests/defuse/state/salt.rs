use defuse::contract::Role;

use defuse_test_utils::asserts::ResultAssertsExt;
use rstest::rstest;

use crate::{
    tests::defuse::{env::Env, state::SaltManagerExt},
    utils::acl::AclExt,
};

#[tokio::test]
#[rstest]
async fn update_current_salt() {
    let mut env = Env::builder().deployer_as_super_admin().build().await;
    let prev_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

    let user1 = env.get_or_create_user().await;
    let user2 = env.get_or_create_user().await;

    // only DAO or salt manager can rotate salt
    {
        user2
            .update_current_salt(env.defuse.id())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // rotate salt by salt manager
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user1.id())
            .await
            .expect("failed to grant role");

        let new_salt = user1
            .update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

        assert_ne!(prev_salt, current_salt);
        assert_eq!(new_salt, current_salt);
        assert!(
            env.defuse
                .is_valid_salt(env.defuse.id(), &prev_salt)
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
#[rstest]
async fn invalidate_salts() {
    let mut env = Env::builder().deployer_as_super_admin().build().await;
    let mut current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();
    let mut prev_salt = current_salt;

    let user1 = env.get_or_create_user().await;
    let user2 = env.get_or_create_user().await;

    // only DAO or salt manager can invalidate salt
    {
        user2
            .invalidate_salts(env.defuse.id(), &[prev_salt])
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // invalidate prev salt by salt manager
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user1.id())
            .await
            .expect("failed to grant role");

        current_salt = user1
            .update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        user1
            .invalidate_salts(env.defuse.id(), &[prev_salt])
            .await
            .expect("unable to rotate salt");

        assert!(
            !env.defuse
                .is_valid_salt(env.defuse.id(), &prev_salt)
                .await
                .unwrap()
        );
    }

    // invalidate current salt by salt manager
    {
        prev_salt = current_salt;
        current_salt = user1
            .invalidate_salts(env.defuse.id(), &[current_salt])
            .await
            .expect("unable to rotate salt");

        assert!(
            !env.defuse
                .is_valid_salt(env.defuse.id(), &prev_salt)
                .await
                .unwrap()
        );
        assert_ne!(prev_salt, current_salt);
    }
}
