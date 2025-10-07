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
    let env = Env::builder().deployer_as_super_admin().build().await;
    let prev_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

    // only DAO or salt manager can rotate salt
    {
        env.user2
            .update_current_salt(env.defuse.id())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // rotate salt by salt manager
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, env.user1.id())
            .await
            .expect("failed to grant role");

        let new_salt = env
            .user1
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
    let env = Env::builder().deployer_as_super_admin().build().await;
    let mut current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();
    let mut prev_salt = current_salt;

    // only DAO or salt manager can invalidate salt
    {
        env.user2
            .invalidate_salts(env.defuse.id(), &[prev_salt])
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // invalidate prev salt by salt manager
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, env.user1.id())
            .await
            .expect("failed to grant role");

        current_salt = env
            .user1
            .update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        env.user1
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
        current_salt = env
            .user1
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
