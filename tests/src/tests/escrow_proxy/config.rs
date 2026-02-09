use defuse_escrow_proxy::ProxyConfig;
use crate::extensions::escrow_proxy::EscrowProxyExt;
use defuse_sandbox::Sandbox;
use near_sdk::serde_json::json;
use near_sdk::{AccountId, GlobalContractId};

#[tokio::test]
async fn escrow_proxy_deployment_and_config() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let escrow_proxy_global = root.deploy_escrow_proxy_global("escrow_proxy_global").await;

    let owner_id = root.sub_account("owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id: owner_id.clone(),
        oneshot_condvar_global_id: GlobalContractId::AccountId(
            root.sub_account("oneshot_condvar_global_id")
                .unwrap()
                .id()
                .clone(),
        ),
        on_auth_caller: root.sub_account("auth_contract").unwrap().id().clone(),
        notifier_id: root.sub_account("notifier").unwrap().id().clone(),
    };

    let proxy_id = root
        .deploy_escrow_proxy_instance(escrow_proxy_global, config.clone())
        .await;
    let actual_config: ProxyConfig = sandbox
        .account(&proxy_id)
        .call_view_function_json("ep_config", json!({}))
        .await
        .unwrap();

    assert_eq!(actual_config, config, "config should match");
}

#[tokio::test]
async fn owner_configuration() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let escrow_proxy_global = root.deploy_escrow_proxy_global("escrow_proxy_global").await;

    let owner_id = root.sub_account("owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(
            root.sub_account("oneshot_condvar_global_id")
                .unwrap()
                .id()
                .clone(),
        ),
        on_auth_caller: root.sub_account("auth_contract").unwrap().id().clone(),
        notifier_id: root.sub_account("notifier").unwrap().id().clone(),
    };

    let _proxy_id = root
        .deploy_escrow_proxy_instance(escrow_proxy_global, config)
        .await;
}
