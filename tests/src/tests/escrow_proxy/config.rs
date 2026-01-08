use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_sandbox::Sandbox;
use defuse_sandbox_ext::EscrowProxyExt;
use near_sdk::{AccountId, GlobalContractId, NearToken};
use std::collections::{HashMap, HashSet};

const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn escrow_proxy_deployment_and_config() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    // Get the proxy account ID (will be created during deployment)
    let proxy = root
        .generate_subaccount("proxy", INIT_BALANCE)
        .await
        .unwrap();

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(
            root.sub_account("per_fill_contract_id")
                .unwrap()
                .id()
                .clone(),
        ),
        escrow_swap_contract_id: GlobalContractId::AccountId(
            root.sub_account("escrow_swap_contract_id")
                .unwrap()
                .id()
                .clone(),
        ),
        auth_contract: root.sub_account("auth_contract").unwrap().id().clone(),
        auth_collee: root.sub_account("auth_collee").unwrap().id().clone(),
    };

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    proxy
        .deploy_escrow_proxy(roles, config.clone())
        .await
        .unwrap();
    let actual_config = proxy.get_escrow_proxy_config().await.unwrap();

    assert_eq!(actual_config, config, "config should match");
}

#[tokio::test]
async fn dao_role_configuration() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let (dao, proxy_account) = futures::try_join!(
        root.generate_subaccount("dao", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(
            root.sub_account("per_fill_contract_id")
                .unwrap()
                .id()
                .clone(),
        ),
        escrow_swap_contract_id: GlobalContractId::AccountId(
            root.sub_account("escrow_swap_contract_id")
                .unwrap()
                .id()
                .clone(),
        ),
        auth_contract: root.sub_account("auth_contract").unwrap().id().clone(),
        auth_collee: root.sub_account("auth_collee").unwrap().id().clone(),
    };

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id().clone()]),
        admins: HashMap::new(),
        grantees: [(defuse_escrow_proxy::Role::DAO, [dao.id().clone()].into())].into(),
    };

    proxy_account
        .deploy_escrow_proxy(roles, config)
        .await
        .unwrap();
}
