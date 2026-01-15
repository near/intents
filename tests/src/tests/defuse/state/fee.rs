use defuse_sandbox::extensions::defuse::contract::{contract::Role, core::fees::Pips};

use defuse_sandbox::extensions::defuse::state::{FeesManagerExt, FeesManagerViewExt};

use crate::{env::Env, sandbox::extensions::acl::AclExt, utils::asserts::ResultAssertsExt};
use near_sdk::AccountId;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn set_fee() {
    let env = Env::builder().deployer_as_super_admin().build().await;
    let prev_fee = env.defuse.fee().await.unwrap();
    let fee = Pips::from_pips(100).unwrap();

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

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

        let current_fee = env.defuse.fee().await.unwrap();

        assert_ne!(prev_fee, current_fee);
        assert_eq!(current_fee, fee);
    }
}

#[rstest]
#[tokio::test]
async fn set_fee_collector() {
    let env = Env::builder().deployer_as_super_admin().build().await;
    let fee_collector: AccountId = "fee-collector.near".to_string().parse().unwrap();

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

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

        let current_collector = env.defuse.fee_collector().await.unwrap();

        assert_eq!(current_collector, fee_collector);
    }
}
