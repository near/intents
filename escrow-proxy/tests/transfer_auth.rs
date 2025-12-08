mod env;

use std::{collections::{HashMap, HashSet}, hash::Hash};

use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_sandbox::{Account, Sandbox};
use defuse_transfer_auth::ext::{TransferAuthAccountExt, DefuseAccountExt};
use env::AccountExt;
use near_sdk::NearToken;

const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn test_deploy_transfer_auth_global_contract() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();


    let wnear = root.deploy_wnear("wnear").await;
    let (transfer_auth_global,  defuse) = futures::join!(
        root.deploy_transfer_auth("global_transfer_auth"),
        root.deploy_verifier("defuse", wnear.id().clone()),
    );


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

    let proxy = sandbox.root().create_subaccount("proxy", INIT_BALANCE).await.unwrap();
    let proxy = proxy.deploy_escrow_proxy(roles, config).await.unwrap();

}
