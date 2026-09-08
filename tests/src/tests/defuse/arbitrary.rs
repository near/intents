use defuse_near_promise::actions::{FunctionCall, NearAction, Transfer};
use defuse_sandbox::{
    extensions::{
        acl::AccessControllableExt,
        defuse::{DefuseExt, contract::Role},
    },
    kit::NearToken,
};
use near_gas::NearGas;
use near_sdk::json_types::U128;
use serde_json::json;

use crate::tests::defuse::env::{Env, env};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn transfer_ft_with_arbitrary_call(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let amount = 1000;

    let (admin, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![env.defuse.contract_id(), admin.account_id()],
        vec![ft.contract_id()],
    )
    .await;

    ft.transfer(env.defuse.contract_id(), amount)
        .await
        .expect("Failed to transfer tokens to defuse");

    assert_eq!(
        ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
        amount
    );
    assert_eq!(ft.balance_of(admin.account_id()).await.unwrap().raw(), 0);

    let deposit = NearToken::from_yoctonear(1);
    let actions = NearAction::FunctionCall(FunctionCall {
        function_name: "ft_transfer".to_string(),
        args: json!({
            "receiver_id": admin.account_id(),
            "amount": U128(amount),
            "memo": "arbitrary call transfer".to_string(),
        })
        .to_string()
        .into_bytes(),
        deposit: NearToken::from_yoctonear(1),
        gas: NearGas::from_tgas(100),
        gas_weight: 1,
    });

    admin
        .defuse_arbitrary_call(
            env.defuse.contract_id(),
            ft.contract_id(),
            &actions,
            &deposit,
        )
        .await
        .expect_err("Insufficient permissions for method");

    // grant DAO role
    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let contract_balance_before = env.balance(env.defuse.contract_id()).await.unwrap();

    admin
        .defuse_arbitrary_call(
            env.defuse.contract_id(),
            ft.contract_id(),
            &actions,
            &deposit,
        )
        .await
        .unwrap();

    let contract_balance_after = env.balance(env.defuse.contract_id()).await.unwrap();

    assert_eq!(
        ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
        0
    );
    assert_eq!(
        ft.balance_of(admin.account_id()).await.unwrap().raw(),
        amount
    );
    assert!(contract_balance_after.total >= contract_balance_before.total);
}

#[rstest]
#[tokio::test]
async fn transfer_near_with_arbitrary_call(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let amount = NearToken::from_near(1);
    let deposit = NearToken::from_near(0);

    let (admin, receiver) = futures::join!(env.create_user(), env.create_user());

    let action = NearAction::Transfer(Transfer { amount });

    admin
        .defuse_arbitrary_call(
            env.defuse.contract_id(),
            receiver.account_id(),
            &action,
            &deposit,
        )
        .await
        .expect_err("Insufficient permissions for method");

    let receiver_balance_before = env.balance(receiver.account_id()).await.unwrap().total;

    // grant DAO role
    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    admin
        .defuse_arbitrary_call(
            env.defuse.contract_id(),
            receiver.account_id(),
            &action,
            &deposit,
        )
        .await
        .unwrap();

    let receiver_balance_after = env.balance(receiver.account_id()).await.unwrap().total;

    assert_eq!(
        receiver_balance_after,
        receiver_balance_before.saturating_add(amount)
    );
}

#[rstest]
#[tokio::test]
async fn arbitrary_call_with_gas_exceeding_action(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let amount = 1000;

    let (admin, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![env.defuse.contract_id(), admin.account_id()],
        vec![ft.contract_id()],
    )
    .await;

    ft.transfer(env.defuse.contract_id(), amount)
        .await
        .expect("Failed to transfer tokens to defuse");

    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let action = NearAction::FunctionCall(FunctionCall {
        function_name: "ft_transfer".to_string(),
        args: json!({
            "receiver_id": admin.account_id(),
            "amount": U128(amount),
            "memo": "arbitrary call transfer".to_string(),
        })
        .to_string()
        .into_bytes(),
        deposit: NearToken::from_yoctonear(1),
        gas: NearGas::from_tgas(500),
        gas_weight: 1,
    });

    admin
        .defuse_arbitrary_call(
            env.defuse.contract_id(),
            ft.contract_id(),
            &action,
            &NearToken::from_yoctonear(1),
        )
        .await
        .expect_err("action requires more gas than attached to the transaction");

    // the whole call is rolled back, so nothing moved
    assert_eq!(
        ft.balance_of(env.defuse.contract_id()).await.unwrap().raw(),
        amount
    );
    assert_eq!(ft.balance_of(admin.account_id()).await.unwrap().raw(), 0);
}
