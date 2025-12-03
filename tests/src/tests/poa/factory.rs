use defuse_poa_factory::{contract::Role, extensions::PoAFactoryExt};
use defuse_sandbox::{
    Sandbox,
    extensions::ft::{FtExt, FtViewExt},
};
use futures::try_join;
use rstest::rstest;
use std::sync::LazyLock;

use crate::utils::read_wasm;

static POA_FACTORY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse_poa_factory"));

#[tokio::test]
#[rstest]
async fn deploy_mint() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();
    let user = sandbox
        .create_account("user1")
        .await
        .expect("Failed to create user");

    let poa_factory = root
        .deploy_poa_factory(
            "poa-factory",
            POA_FACTORY_WASM.clone(),
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

    assert_eq!(root.ft_balance_of(&ft1, user.id()).await.unwrap(), 0);

    try_join!(
        root.ft_storage_deposit(&ft1, Some(root.id())),
        root.ft_storage_deposit(&ft1, Some(user.id()))
    )
    .unwrap();

    user.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
        .await
        .unwrap_err();

    root.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
        .await
        .unwrap();

    assert_eq!(root.ft_balance_of(&ft1, user.id()).await.unwrap(), 1000);
}
