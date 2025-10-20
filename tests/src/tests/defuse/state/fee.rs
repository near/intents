use defuse::{contract::Role, core::fees::Pips};

use defuse_test_utils::asserts::ResultAssertsExt;
use near_sdk::AccountId;
use rstest::rstest;

use crate::{
    tests::defuse::{env::Env, state::FeesManagerExt},
    utils::acl::AclExt,
};

#[tokio::test]
#[rstest]
async fn set_fee() {
    let mut env = Env::builder().deployer_as_super_admin().build().await;
    let prev_fee = env.defuse.fee(env.defuse.id()).await.unwrap();
    let fee = Pips::from_pips(100).unwrap();

    let user1 = env.get_or_create_user().await;
    let user2 = env.get_or_create_user().await;

    // only DAO or fee manager can set fee
    {
        user2
            .set_fee(env.defuse.id(), fee)
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // set fee by fee manager
    {
        env.acl_grant_role(env.defuse.id(), Role::FeesManager, user1.id())
            .await
            .expect("failed to grant role");

        user1
            .set_fee(env.defuse.id(), fee)
            .await
            .expect("unable to set fee");

        let current_fee = env.defuse.fee(env.defuse.id()).await.unwrap();

        assert_ne!(prev_fee, current_fee);
        assert_eq!(current_fee, fee);
    }
}

#[tokio::test]
#[rstest]
async fn set_fee_collector() {
    let mut env = Env::builder().deployer_as_super_admin().build().await;
    let fee_collector: AccountId = "fee-collector.near".to_string().parse().unwrap();

    let user1 = env.get_or_create_user().await;
    let user2 = env.get_or_create_user().await;

    // only DAO or fee manager can set fee collector
    {
        user2
            .set_fee_collector(env.defuse.id(), &fee_collector)
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // set fee by fee manager
    {
        env.acl_grant_role(env.defuse.id(), Role::FeesManager, user1.id())
            .await
            .expect("failed to grant role");

        user1
            .set_fee_collector(env.defuse.id(), &fee_collector)
            .await
            .expect("unable to set fee");

        let current_collector = env.defuse.fee_collector(env.defuse.id()).await.unwrap();

        assert_eq!(current_collector, fee_collector);
    }
}
