use crate::{
    sandbox::extensions::acl::AclExt, tests::defuse::env::Env, utils::asserts::ResultAssertsExt,
};
use defuse::core::events::DefuseEvent;
use defuse::core::fees::{FeeChangedEvent, FeeCollectorChangedEvent};
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::defuse::contract::{contract::Role, core::fees::Pips};
use defuse_sandbox::extensions::defuse::state::{FeesManagerExt, FeesManagerViewExt};
use near_sdk::{AccountId, AsNep297Event};
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

        let res = user1
            .set_fee(env.defuse.id(), fee)
            .await
            .expect("unable to set fee");

        let event = DefuseEvent::FeeChanged(FeeChangedEvent {
            old_fee: prev_fee,
            new_fee: fee,
        })
        .to_nep297_event()
        .to_event_log();

        assert_a_contains_b!(
            a: res.logs().clone(),
            b: [event]
        );

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

        let res = user1
            .set_fee_collector(env.defuse.id(), &fee_collector)
            .await
            .expect("unable to set fee");

        let event = DefuseEvent::FeeCollectorChanged(FeeCollectorChangedEvent {
            old_fee_collector: env.root().id().into(),
            new_fee_collector: fee_collector.clone().into(),
        })
        .to_nep297_event()
        .to_event_log();

        assert_a_contains_b!(
            a: res.logs().clone(),
            b: [event]
        );

        let current_collector = env.defuse.fee_collector().await.unwrap();

        assert_eq!(current_collector, fee_collector);
    }
}
