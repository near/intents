// FIXME: remove this
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::{
    tests::poa::factory::factory_env::PoAFactoryExt,
    utils::{Sandbox, account::AccountExt, ft::FtExt},
};
use defuse_poa_factory::contract::Role;
use near_sdk::{AccountId, NearToken};
use near_workspaces::{Account, Contract};

const UNVERSIONED_POA_FACTORY_CONTRACT_WASM_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/old-artifacts/unversioned-poa/defuse_poa_factory.wasm"
));

async fn deploy_unversioned_poa_factory(
    deployer_account: &mut Account,
    id: &str,
    super_admins: impl IntoIterator<Item = AccountId>,
    admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
) -> anyhow::Result<Contract> {
    let contract = deployer_account
        .deploy_contract(id, UNVERSIONED_POA_FACTORY_CONTRACT_WASM_BYTES)
        .await?;

    deployer_account
        .transfer_near(contract.id(), NearToken::from_near(100))
        .await?
        .into_result()?;
    contract
        .call("new")
        .args_json(serde_json::json_internal!({
    "super_admins":super_admins.into_iter().collect::<HashSet<_>>(),"admins":admins.into_iter().map(|(role,admins)|(role,admins.into_iter().collect::<HashSet<_>>())).collect::<HashMap<_,_>>(),"grantees":grantees.into_iter().map(|(role,grantees)|(role,grantees.into_iter().collect::<HashSet<_>>())).collect::<HashMap<_,_>>(),
}))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(contract)
}

#[tokio::test]
async fn test_deploy_then_upgrade() {
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
