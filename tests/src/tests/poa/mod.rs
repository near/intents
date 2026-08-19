use defuse_sandbox::{
    account::Account,
    extensions::poa::{
        PoAFactoryExt, PoaFactoryClient, PoaFactoryDeployerExt, PoaGetWithdrawArgs, Withdrawal,
        contract::Role,
    },
    kit::{Near, NearToken},
    root,
};
use defuse_test_utils::wasms::POA_FACTORY_WASM;
use futures::try_join;
use near_sdk::json_types::Base64VecU8;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn deploy_mint(#[future(awt)] root: Near) {
    let user = root
        .create_subaccount("user1", NearToken::from_near(10))
        .await;

    let poa_factory = root
        .deploy_poa_factory(
            "poa-factory",
            [root.account_id().clone()],
            [
                (Role::TokenDeployer, [root.account_id().clone()]),
                (Role::TokenDepositer, [root.account_id().clone()]),
            ],
            [
                (Role::TokenDeployer, [root.account_id().clone()]),
                (Role::TokenDepositer, [root.account_id().clone()]),
            ],
            POA_FACTORY_WASM.clone(),
        )
        .await;

    user.poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap_err();

    root.poa_factory_deploy_token(poa_factory.contract_id(), "ft1.abc", None)
        .await
        .unwrap_err();

    let ft1 = root
        .poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap();

    root.poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap_err();

    assert!(ft1.balance_of(user.account_id()).await.unwrap().is_zero());

    try_join!(
        ft1.storage_deposit(root.account_id(), NearToken::from_near(1))
            .into_future(),
        ft1.storage_deposit(user.account_id(), NearToken::from_near(1))
            .into_future()
    )
    .unwrap();

    user.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "deposit-unauthorized",
        "ft1",
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap_err();

    root.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "deposit-1",
        "ft1",
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let balance: u128 = ft1.balance_of(user.account_id()).await.unwrap().into();

    assert_eq!(balance, 1000);
}

async fn deploy_factory_with_all_roles(root: &Near) -> PoaFactoryClient {
    root.deploy_poa_factory(
        "poa-factory",
        [root.account_id().clone()],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
            (Role::TokenWithdrawer, [root.account_id().clone()]),
        ],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
            (Role::TokenWithdrawer, [root.account_id().clone()]),
        ],
        POA_FACTORY_WASM.clone(),
    )
    .await
}

fn sample_withdrawal(payload_hash: Vec<u8>, metadata: &str) -> Withdrawal {
    Withdrawal {
        chain_id: "eth:1".to_string(),
        payload_hash: Base64VecU8::from(payload_hash),
        timestamp: 1_700_000_000,
        metadata: metadata.to_string(),
    }
}

#[rstest]
#[tokio::test]
async fn ft_deposit_duplicate_id_fails(#[future(awt)] root: Near) {
    let user = root
        .create_subaccount("user2", NearToken::from_near(10))
        .await;

    let poa_factory = deploy_factory_with_all_roles(&root).await;

    let ft = root
        .poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap();

    try_join!(
        ft.storage_deposit(root.account_id(), NearToken::from_near(1))
            .into_future(),
        ft.storage_deposit(user.account_id(), NearToken::from_near(1))
            .into_future()
    )
    .unwrap();

    root.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "same-deposit",
        "ft1",
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let err = root
        .poa_factory_ft_deposit(
            poa_factory.contract_id(),
            "same-deposit",
            "ft1",
            user.account_id(),
            500,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("deposit already exists"),
        "unexpected error: {err:?}"
    );

    let balance: u128 = ft.balance_of(user.account_id()).await.unwrap().into();
    assert_eq!(
        balance, 1000,
        "second deposit must not have credited tokens"
    );

    root.poa_factory_remove_deposits(poa_factory.contract_id(), vec!["same-deposit".to_string()])
        .await
        .unwrap();

    root.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "same-deposit",
        "ft1",
        user.account_id(),
        500,
        None,
        None,
    )
    .await
    .unwrap();

    let balance: u128 = ft.balance_of(user.account_id()).await.unwrap().into();
    assert_eq!(balance, 1500);
}

#[rstest]
#[tokio::test]
async fn ft_withdraw_records_and_rejects_duplicate(#[future(awt)] root: Near) {
    let unauthorized = root
        .create_subaccount("unauth", NearToken::from_near(10))
        .await;
    let poa_factory = deploy_factory_with_all_roles(&root).await;

    let withdrawal = sample_withdrawal(vec![1, 2, 3, 4], "meta-1");

    unauthorized
        .poa_factory_ft_withdraw(poa_factory.contract_id(), "w-1", withdrawal.clone())
        .await
        .unwrap_err();

    root.poa_factory_ft_withdraw(poa_factory.contract_id(), "w-1", withdrawal.clone())
        .await
        .unwrap();

    let stored = poa_factory
        .get_withdraw(PoaGetWithdrawArgs {
            withdrawal_id: "w-1".to_string(),
        })
        .await
        .unwrap()
        .expect("withdrawal must be stored");
    assert_eq!(stored.chain_id, withdrawal.chain_id);
    assert_eq!(stored.payload_hash.0, withdrawal.payload_hash.0);
    assert_eq!(stored.timestamp, withdrawal.timestamp);
    assert_eq!(stored.metadata, withdrawal.metadata);

    let err = root
        .poa_factory_ft_withdraw(poa_factory.contract_id(), "w-1", withdrawal.clone())
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("withdrawal already exists"),
        "unexpected error: {err:?}"
    );

    root.poa_factory_remove_withdraws(poa_factory.contract_id(), vec!["w-1".to_string()])
        .await
        .unwrap();
    assert!(
        poa_factory
            .get_withdraw(PoaGetWithdrawArgs {
                withdrawal_id: "w-1".to_string(),
            })
            .await
            .unwrap()
            .is_none()
    );
}

#[rstest]
#[tokio::test]
async fn ft_update_withdraw(#[future(awt)] root: Near) {
    let unauthorized = root
        .create_subaccount("unauth-upd", NearToken::from_near(10))
        .await;
    let poa_factory = deploy_factory_with_all_roles(&root).await;

    let original = sample_withdrawal(vec![9, 9, 9], "meta-orig");
    root.poa_factory_ft_withdraw(poa_factory.contract_id(), "w-upd", original.clone())
        .await
        .unwrap();

    let updated_hash = Base64VecU8::from(vec![5, 5, 5]);
    let updated_metadata = "meta-updated".to_string();

    unauthorized
        .poa_factory_ft_update_withdraw(
            poa_factory.contract_id(),
            "w-upd",
            original.payload_hash.clone(),
            updated_hash.clone(),
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();

    let wrong_prev = Base64VecU8::from(vec![0, 0, 0]);
    let err = root
        .poa_factory_ft_update_withdraw(
            poa_factory.contract_id(),
            "w-upd",
            wrong_prev,
            updated_hash.clone(),
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("payload hash mismatch"),
        "unexpected error: {err:?}"
    );

    let err = root
        .poa_factory_ft_update_withdraw(
            poa_factory.contract_id(),
            "missing-id",
            original.payload_hash.clone(),
            updated_hash.clone(),
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("withdrawal not found"),
        "unexpected error: {err:?}"
    );

    root.poa_factory_ft_update_withdraw(
        poa_factory.contract_id(),
        "w-upd",
        original.payload_hash.clone(),
        updated_hash.clone(),
        updated_metadata.clone(),
    )
    .await
    .unwrap();

    let stored = poa_factory
        .get_withdraw(PoaGetWithdrawArgs {
            withdrawal_id: "w-upd".to_string(),
        })
        .await
        .unwrap()
        .expect("withdrawal must still exist");
    assert_eq!(stored.chain_id, original.chain_id);
    assert_eq!(stored.timestamp, original.timestamp);
    assert_eq!(stored.payload_hash.0, updated_hash.0);
    assert_eq!(stored.metadata, updated_metadata);
}
