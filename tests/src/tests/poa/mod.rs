use defuse_sandbox::{
    account::Account,
    extensions::poa::{PoAFactoryExt, PoaFactoryDeployerExt, contract::Role},
    kit::Near,
    root,
};
use defuse_test_utils::wasms::POA_FACTORY_WASM;
use futures::try_join;
use near_sdk::NearToken;
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
