use defuse_escrow_proxy::ProxyConfig;
use defuse_sandbox::{EscrowProxyExt, Sandbox};
use near_sdk::{AccountId, GlobalContractId, NearToken};

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
        owner: proxy.id().clone(),
        oneshot_condvar_global_id: GlobalContractId::AccountId(
            root.sub_account("oneshot_condvar_global_id")
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
        notifier: root.sub_account("notifier").unwrap().id().clone(),
    };

    proxy.deploy_escrow_proxy(config.clone()).await.unwrap();
    let actual_config = proxy.get_escrow_proxy_config().await.unwrap();

    assert_eq!(actual_config, config, "config should match");
}

#[tokio::test]
async fn owner_configuration() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    let (owner, proxy_account) = futures::try_join!(
        root.generate_subaccount("owner", INIT_BALANCE),
        root.generate_subaccount("proxy", INIT_BALANCE),
    )
    .unwrap();

    let config = ProxyConfig {
        owner: owner.id().clone(),
        oneshot_condvar_global_id: GlobalContractId::AccountId(
            root.sub_account("oneshot_condvar_global_id")
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
        notifier: root.sub_account("notifier").unwrap().id().clone(),
    };

    proxy_account.deploy_escrow_proxy(config).await.unwrap();
}
