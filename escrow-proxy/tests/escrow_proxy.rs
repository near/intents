mod env;

use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use env::{AccountExt, BaseEnv};
use near_sdk::{Gas, NearToken};
use serde_json::json;
use std::collections::{HashMap, HashSet};

const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn escrow_proxy_deployment_and_config() {
    let env = BaseEnv::new().await.unwrap();
    let root = env.root();

    // Get the proxy account ID (will be created during deployment)
    let proxy = root.create_subaccount("proxy", INIT_BALANCE).await.unwrap();

    let config = ProxyConfig {
        per_fill_global_contract_id: root.subaccount("per_fill_global_contract_id").id().clone(),
        escrow_swap_global_contract_id: root
            .subaccount("escrow_swap_global_contract_id")
            .id()
            .clone(),
        auth_contract: root.subaccount("auth_contract").id().clone(),
        auth_collee: root.subaccount("auth_collee").id().clone(),
    };

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();
    let actual_config = proxy
        .get_escrow_proxy_config()
        .await
        .unwrap();

    assert_eq!(actual_config, config, "config should match");
}

#[tokio::test]
async fn dao_can_upgrade_contract() {
    let env = BaseEnv::new().await.unwrap();
    let root = env.root();

    let (dao, proxy_account, alice) = futures::try_join!(
        root.create_subaccount("dao", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
        root.create_subaccount("alice", INIT_BALANCE)
    )
    .unwrap();

    let config = ProxyConfig {
        per_fill_global_contract_id: root.subaccount("per_fill_global_contract_id").id().clone(),
        escrow_swap_global_contract_id: root
            .subaccount("escrow_swap_global_contract_id")
            .id()
            .clone(),
        auth_contract: root.subaccount("auth_contract").id().clone(),
        auth_collee: root.subaccount("auth_collee").id().clone(),
    };

    let mut grantees = HashMap::new();
    grantees.insert(defuse_escrow_proxy::Role::DAO, HashSet::from([dao.id()]));

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id()]),
        admins: HashMap::new(),
        grantees: [(defuse_escrow_proxy::Role::DAO, [dao.id()].into())].into()
    };

    proxy_account.deploy_escrow_proxy(roles, config).await.unwrap();

}
