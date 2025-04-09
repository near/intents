use defuse_poa_factory::contract::Role;

use crate::{
    tests::poa::factory::factory_env::PoAFactoryExt,
    utils::{Sandbox, ft::FtExt},
};

#[tokio::test]
async fn test_deploy_mint() {
    let sandbox = Sandbox::new().await.unwrap();
    let root = sandbox.root_account();
    let user = sandbox.create_account("user1").await;

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
        .unwrap_err()
        .to_string();

    assert!(
        root.poa_factory_deploy_token(poa_factory.id(), "ft1.abc", None)
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid token name")
    );

    let ft1 = root
        .poa_factory_deploy_token(poa_factory.id(), "ft1", None)
        .await
        .unwrap();

    assert!(
        root.poa_factory_deploy_token(poa_factory.id(), "ft1", None)
            .await
            .unwrap_err()
            .to_string()
            .contains("token exists")
    );

    assert_eq!(
        sandbox.ft_token_balance_of(&ft1, user.id()).await.unwrap(),
        0
    );

    sandbox
        .ft_storage_deposit_many(&ft1, &[root.id(), user.id()])
        .await
        .unwrap();

    assert!(
        user.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
            .await
            .unwrap_err()
            .to_string()
            .contains("Requires one of these roles")
    );

    root.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
        .await
        .unwrap();

    assert_eq!(
        sandbox.ft_token_balance_of(&ft1, user.id()).await.unwrap(),
        1000
    );
}
