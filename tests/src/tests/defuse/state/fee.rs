use crate::{
    tests::defuse::env::{Env, env},
    utils::asserts::ResultAssertsExt,
};
use defuse_sandbox::{
    extensions::{
        acl::AccessControllableExt,
        defuse::{
            DefuseExt,
            contract::Role,
            core::{
                events::DefuseEvent,
                fees::{FeeChangedEvent, FeeCollectorChangedEvent, Pips},
            },
        },
    },
    kit::AccountId,
};
use futures::FutureExt;
use near_sdk_core::events::AsNep297Event;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn set_fee(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let fee = Pips::from_pips(100).unwrap();

    let (prev_fee, user1, user2) = futures::try_join!(
        env.defuse.fee().into_future(),
        env.create_user().map(Ok),
        env.create_user().map(Ok),
    )
    .unwrap();

    // only DAO or fee manager can set fee
    {
        user2
            .defuse_set_fee(env.defuse.contract_id().clone(), fee)
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // set fee by fee manager
    {
        env.acl_grant_role(
            env.defuse.contract_id().clone(),
            Role::FeesManager,
            user1.account_id().clone(),
        )
        .await
        .expect("failed to grant role");

        let res = user1
            .defuse_set_fee(env.defuse.contract_id().clone(), fee)
            .await
            .expect("unable to set fee");

        let event = DefuseEvent::FeeChanged(FeeChangedEvent {
            old_fee: prev_fee,
            new_fee: fee,
        })
        .to_nep297_event()
        .to_event_log();

        assert!(res.logs().contains(&event));

        let current_fee = env.defuse.fee().await.unwrap();

        assert_ne!(prev_fee, current_fee);
        assert_eq!(current_fee, fee);
    }
}

#[rstest]
#[tokio::test]
async fn set_fee_collector(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let fee_collector: AccountId = "fee-collector.near".to_string().parse().unwrap();

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    // only DAO or fee manager can set fee collector
    {
        user2
            .defuse_set_fee_collector(env.defuse.contract_id().clone(), fee_collector.clone())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // set fee by fee manager
    {
        env.acl_grant_role(
            env.defuse.contract_id().clone(),
            Role::FeesManager,
            user1.account_id().clone(),
        )
        .await
        .expect("failed to grant role");

        let res = user1
            .defuse_set_fee_collector(env.defuse.contract_id().clone(), fee_collector.clone())
            .await
            .expect("unable to set fee");

        let event = DefuseEvent::FeeCollectorChanged(FeeCollectorChangedEvent {
            old_fee_collector: env.account_id().clone().into(),
            new_fee_collector: fee_collector.clone().into(),
        })
        .to_nep297_event()
        .to_event_log();

        assert!(res.logs().contains(&event));

        let current_collector = env.defuse.fee_collector().await.unwrap();

        assert_eq!(current_collector, fee_collector);
    }
}
