use defuse::arbitrary::ArbitraryAction;
use defuse_actions::FunctionCallAction;
use defuse_sandbox::{FtExt, extensions::defuse::arbitrary::ArbitraryManagerExt};
use defuse_sandbox::{FtViewExt, extensions::defuse::contract::contract::Role};
use near_sdk::{Gas, NearToken, json_types::U128};
use serde_json::json;

use crate::{sandbox::extensions::acl::AclExt, tests::defuse::env::Env};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn transfer_ft_with_arbitrary_call() {
    let amount = 1000;
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (admin, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![env.defuse.id(), admin.id()], vec![ft.id()])
        .await;

    env.ft_transfer(ft.id(), env.defuse.id(), amount, None)
        .await
        .expect("Failed to transfer tokens to defuse");

    assert_eq!(ft.ft_balance_of(env.defuse.id()).await.unwrap(), amount);
    assert_eq!(ft.ft_balance_of(admin.id()).await.unwrap(), 0);

    let deposit = NearToken::from_yoctonear(1);
    let actions = ArbitraryAction::FunctionCall(FunctionCallAction {
        function_name: "ft_transfer".to_string(),
        args: json!({
            "receiver_id": admin.id(),
            "amount": U128(amount),
            "memo": "arbitrary call transfer".to_string(),
        })
        .to_string()
        .into_bytes(),
        deposit: NearToken::from_yoctonear(1),
        min_gas: Gas::from_gas(0),
        gas_weight: 1,
    });

    admin
        .arbitrary_call(env.defuse.id(), ft.id(), actions.clone(), deposit)
        .await
        .expect_err("Insufficient permissions for method");

    // grant DAO role
    env.acl_grant_role(env.defuse.id(), Role::DAO, admin.id())
        .await
        .unwrap();

    let contract_balance_before = env.near_balance(&env.defuse).await;

    admin
        .arbitrary_call(env.defuse.id(), ft.id(), actions, deposit)
        .await
        .unwrap();

    let contract_balance_after = env.near_balance(&env.defuse).await;

    assert_eq!(ft.ft_balance_of(env.defuse.id()).await.unwrap(), 0);
    assert_eq!(ft.ft_balance_of(admin.id()).await.unwrap(), amount);
    assert!(contract_balance_after >= contract_balance_before);
}

#[rstest]
#[tokio::test]
async fn transfer_near_with_arbitrary_call() {
    let deposit = NearToken::from_yoctonear(1);
    let amount = NearToken::from_near(1);
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (admin, receiver) = futures::join!(env.create_user(), env.create_user());

    let action = ArbitraryAction::Transfer(defuse_actions::TransferAction { amount });

    admin
        .arbitrary_call(env.defuse.id(), receiver.id(), action.clone(), deposit)
        .await
        .expect_err("Insufficient permissions for method");

    let receiver_balance_before = env.near_balance(&receiver).await;

    // grant DAO role
    env.acl_grant_role(env.defuse.id(), Role::DAO, admin.id())
        .await
        .unwrap();

    admin
        .arbitrary_call(env.defuse.id(), receiver.id(), action, deposit)
        .await
        .unwrap();

    let receiver_balance_after = env.near_balance(&receiver).await;

    assert_eq!(
        receiver_balance_after,
        receiver_balance_before.saturating_add(amount)
    );
}
