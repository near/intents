use defuse_poa_factory::{
    contract::Role,
    sandbox_ext::{PoAFactoryDeployerExt, PoAFactoryExt},
};
use defuse_sandbox::{
    Sandbox,
    extensions::{ft::FtViewExt, storage_management::StorageManagementExt},
};
use futures::try_join;
use near_sdk::NearToken;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn deploy_mint() {
    let sandbox = Sandbox::new("test".parse().unwrap()).await;
    let root = sandbox.root();
    let user = root
        .create_subaccount("user1", NearToken::from_near(10))
        .await
        .expect("Failed to create user");

    let poa_factory = root
        .deploy_poa_factory(
            "poa-factory",
            [root.id().clone()],
            [
                (Role::TokenDeployer, [root.id().clone()]),
                (Role::TokenDepositer, [root.id().clone()]),
            ],
            [
                (Role::TokenDeployer, [root.id().clone()]),
                (Role::TokenDepositer, [root.id().clone()]),
            ],
        )
        .await
        .unwrap();

    user.poa_factory_deploy_token(poa_factory.id(), "ft1", None)
        .await
        .unwrap_err();

    root.poa_factory_deploy_token(poa_factory.id(), "ft1.abc", None)
        .await
        .unwrap_err();

    let ft1 = root
        .poa_factory_deploy_token(poa_factory.id(), "ft1", None)
        .await
        .unwrap();

    root.poa_factory_deploy_token(poa_factory.id(), "ft1", None)
        .await
        .unwrap_err();

    assert_eq!(ft1.ft_balance_of(user.id()).await.unwrap(), 0);

    try_join!(
        root.storage_deposit(ft1.id(), Some(root.id()), NearToken::from_near(1)),
        root.storage_deposit(ft1.id(), Some(user.id()), NearToken::from_near(1))
    )
    .unwrap();

    user.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
        .await
        .unwrap_err();

    root.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
        .await
        .unwrap();

    assert_eq!(ft1.ft_balance_of(user.id()).await.unwrap(), 1000);
}
