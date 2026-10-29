use defuse_sandbox::{
    extensions::{
        acl::AccessControllableExt,
        defuse::{
            DefuseExt,
            contract::Role,
            near_promise::{
                NearPromise, StateInitV1,
                actions::{DeterministicStateInit, FunctionCall, NearAction, Transfer},
            },
        },
    },
    kit::{NearToken, protocol::Action, transaction::FunctionCall as KitFunctionCall},
};
use near_sdk::Gas as NearGas;

use crate::{
    tests::defuse::env::{Env, env},
    utils::asserts::ResultAssertsExt,
};
use rstest::rstest;

// TODO: remove when into_fn_call_action is added to CallBuilder
pub fn into_fn_call_action(action: impl TryInto<Action, Error: std::fmt::Debug>) -> NearAction {
    let action = action.try_into().unwrap();
    let Action::FunctionCall(action) = action else {
        panic!("Expected Action::FunctionCall, got {action:?}");
    };
    NearAction::FunctionCall(action.into())
}

#[rstest]
#[tokio::test]
async fn multiple_actions_with_admin_call(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let amount = 1000;

    let (admin, ft1, ft2) =
        futures::join!(env.create_user(), env.create_token(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![env.defuse.contract_id()],
        vec![ft1.contract_id(), ft2.contract_id()],
    )
    .await;

    ft1.transfer(env.defuse.contract_id(), amount, None)
        .await
        .expect("Failed to transfer tokens to defuse")
        .result()
        .expect("Failed to transfer tokens to defuse");
    ft2.transfer(env.defuse.contract_id(), amount, None)
        .await
        .expect("Failed to transfer tokens to defuse")
        .result()
        .expect("Failed to transfer tokens to defuse");

    assert_eq!(
        ft1.balance_of(env.defuse.contract_id()).await.unwrap(),
        amount
    );
    assert_eq!(ft1.balance_of(admin.account_id()).await.unwrap(), 0);
    assert_eq!(
        ft2.balance_of(env.defuse.contract_id()).await.unwrap(),
        amount
    );
    assert_eq!(ft2.balance_of(admin.account_id()).await.unwrap(), 0);

    let deposit = NearToken::from_yoctonear(1);
    let storage = NearToken::from_near(1);

    let first_promise = [NearPromise::new(ft1.contract_id())
        .add_action(into_fn_call_action(KitFunctionCall::from(
            ft1.storage_deposit(admin.account_id(), storage)
                .gas(NearGas::from_tgas(100)),
        )))
        .add_action(into_fn_call_action(KitFunctionCall::from(
            ft1.transfer(admin.account_id(), amount, None)
                .gas(NearGas::from_tgas(100)),
        )))];

    let second_promise = [NearPromise::new(ft2.contract_id())
        .add_action(into_fn_call_action(KitFunctionCall::from(
            ft2.storage_deposit(admin.account_id(), storage)
                .gas(NearGas::from_tgas(100)),
        )))
        .add_action(into_fn_call_action(KitFunctionCall::from(
            ft2.transfer(admin.account_id(), amount, None)
                .gas(NearGas::from_tgas(100)),
        )))];

    let promises = [first_promise, second_promise].concat();

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promises,
            &deposit,
            NearGas::from_tgas(500),
        )
        .await
        .assert_err_contains("Insufficient permissions for method");

    // grant DAO role
    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let defuse_balance_before = env
        .near()
        .balance(env.defuse.contract_id())
        .await
        .unwrap()
        .total;

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promises,
            &deposit,
            NearGas::from_tgas(500),
        )
        .await
        .unwrap();

    let defuse_balance_after = env
        .near()
        .balance(env.defuse.contract_id())
        .await
        .unwrap()
        .total;

    // Both `storage_deposit`s are paid out of the contract's own balance.
    assert!(defuse_balance_after < defuse_balance_before);

    assert_eq!(ft1.balance_of(env.defuse.contract_id()).await.unwrap(), 0);
    assert_eq!(ft1.balance_of(admin.account_id()).await.unwrap(), amount);

    assert_eq!(ft2.balance_of(env.defuse.contract_id()).await.unwrap(), 0);
    assert_eq!(ft2.balance_of(admin.account_id()).await.unwrap(), amount);
}

#[rstest]
#[tokio::test]
async fn transfer_near_with_admin_call(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let amount = NearToken::from_near(1);
    let deposit = NearToken::from_yoctonear(1);

    let (admin, receiver) = futures::join!(env.create_user(), env.create_user());

    let promise = [NearPromise::new(receiver.account_id())
        .add_action(NearAction::Transfer(Transfer { amount }))];

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promise,
            &deposit,
            NearGas::from_tgas(100),
        )
        .await
        .assert_err_contains("Insufficient permissions for method");

    let receiver_balance_before = env
        .near()
        .balance(receiver.account_id())
        .await
        .unwrap()
        .total;

    // grant DAO role
    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promise,
            &deposit,
            NearGas::from_tgas(100),
        )
        .await
        .unwrap();

    let receiver_balance_after = env
        .near()
        .balance(receiver.account_id())
        .await
        .unwrap()
        .total;

    assert_eq!(
        receiver_balance_after,
        receiver_balance_before.saturating_add(amount)
    );
}

#[rstest]
#[tokio::test]
async fn admin_call_with_gas_exceeding_action(
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

    ft.transfer(env.defuse.contract_id(), amount, None)
        .await
        .expect("Failed to transfer tokens to defuse")
        .result()
        .expect("Failed to transfer tokens to defuse");

    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let promise = [
        NearPromise::new(ft.contract_id()).add_action(into_fn_call_action(KitFunctionCall::from(
            ft.transfer(admin.account_id(), amount, None)
                .gas(NearGas::from_tgas(500)),
        ))),
    ];

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promise,
            &NearToken::from_yoctonear(1),
            NearGas::from_tgas(100),
        )
        .await
        .assert_err_contains("Exceeded the prepaid gas");

    // the whole call is rolled back, so nothing moved
    assert_eq!(
        ft.balance_of(env.defuse.contract_id()).await.unwrap(),
        amount
    );
    assert_eq!(ft.balance_of(admin.account_id()).await.unwrap(), 0);
}

#[rstest]
#[tokio::test]
async fn admin_call_accepts_only_allowed_actions(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (admin, receiver) = futures::join!(env.create_user(), env.create_user());

    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let promise =
        [
            NearPromise::new(receiver.account_id()).add_action(NearAction::DeterministicStateInit(
                DeterministicStateInit {
                    state_init: StateInitV1::code(env.defuse.contract_id().to_owned()).into(),
                    deposit: NearToken::from_yoctonear(0),
                },
            )),
        ];

    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promise,
            &NearToken::from_yoctonear(1),
            NearGas::from_tgas(100),
        )
        .await
        .assert_err_contains("unsupported action");
}

#[rstest]
#[tokio::test]
async fn admin_call_refunds_failed_deposit_to_contract(
    #[with(Env::builder().deployer_as_super_admin())]
    #[future(awt)]
    env: Env,
) {
    let (admin, ft) = futures::join!(env.create_user(), env.create_token());

    env.acl_grant_role(env.defuse.contract_id(), Role::DAO, admin.account_id())
        .await
        .unwrap();

    let deposit = NearToken::from_near(1);

    let promise = [
        NearPromise::new(ft.contract_id()).add_action(NearAction::FunctionCall(FunctionCall {
            function_name: "no_such_method".to_string(),
            args: b"{}".to_vec(),
            deposit,
            gas: NearGas::from_tgas(10),
            gas_weight: 0,
        })),
    ];

    let defuse_before = env
        .near()
        .balance(env.defuse.contract_id())
        .await
        .unwrap()
        .total;
    let admin_before = env.near().balance(admin.account_id()).await.unwrap().total;

    // Promises are detached
    admin
        .defuse_admin_call(
            env.defuse.contract_id(),
            &promise,
            &deposit,
            NearGas::from_tgas(100),
        )
        .await
        .unwrap();

    let defuse_after = env
        .near()
        .balance(env.defuse.contract_id())
        .await
        .unwrap()
        .total;
    let admin_after = env.near().balance(admin.account_id()).await.unwrap().total;

    assert!(admin_before.saturating_sub(admin_after) >= deposit);
    assert!(defuse_after.saturating_sub(defuse_before) >= deposit);
}
