mod env;

use std::collections::{HashMap, HashSet};

use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_sandbox::{Account, Sandbox};
use defuse_transfer_auth::ext::{DefuseAccountExt, TransferAuthAccountExt};
use multi_token_receiver_stub::ext::MtReceiverStubAccountExt;
use env::AccountExt;
use near_sdk::NearToken;

const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn test_deploy_transfer_auth_global_contract() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let wnear = root.deploy_wnear("wnear").await;
    let (transfer_auth_global, defuse, mt_receiver_global) = futures::join!(
        root.deploy_transfer_auth("global_transfer_auth"),
        root.deploy_verifier("defuse", wnear.id().clone()),
        root.deploy_mt_receiver_stub_global("mt_receiver_global"),
    );

    // Deploy an instance of mt-receiver-stub referencing the global contract
    let mt_receiver_instance = root
        .deploy_mt_receiver_stub_instance(mt_receiver_global.clone())
        .await;

    let relay = root.create_subaccount("relay", INIT_BALANCE).await.unwrap();

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    let config = ProxyConfig {
        per_fill_global_contract_id: transfer_auth_global.clone(),
        //NOTE: not really used in this test YET
        escrow_swap_global_contract_id: transfer_auth_global.clone(),
        auth_contract: defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };

    let proxy = sandbox
        .root()
        .create_subaccount("proxy", INIT_BALANCE)
        .await
        .unwrap();
    proxy.deploy_escrow_proxy(roles, config).await.unwrap();

    // Verify mt-receiver deployments
    println!("mt_receiver_global: {mt_receiver_global}");
    println!("mt_receiver_instance: {mt_receiver_instance}");
}
