mod env;

use std::{collections::{HashMap, HashSet}, hash::Hash};

use defuse_escrow_proxy::RolesConfig;
use defuse_sandbox::{Account, Sandbox};
use defuse_transfer_auth::ext::{TransferAuthAccountExt, DefuseAccountExt};
use env::AccountExt;

#[tokio::test]
async fn test_deploy_transfer_auth_global_contract() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();


    let (global_contract_id, wnear, defuse) = futures::join!(
        root.deploy_transfer_auth("global_transfer_auth"),
        root.deploy_wnear("wnear"),
        root.deploy_verifier("defuse", wnear.id().clone()),
    );


    let roles = RolesConfig {
        super_admins: HashSet::from([root.id()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };
    let proxy = sandbox.root().deploy_escrow_proxy("proxy").await;

    // let config = ProxyConfig {
    //     per_fill_global_contract_id: root.subaccount("per_fill_global_contract_id").id().clone(),
    //     escrow_swap_global_contract_id: root
    //         .subaccount("escrow_swap_global_contract_id")
    //         .id()
    //         .clone(),
    //     auth_contract: root.subaccount("auth_contract").id().clone(),
    //     auth_collee: root.subaccount("auth_collee").id().clone(),
    // };




    let account = Account::new(global_contract_id, sandbox.root().network_config().clone());
    assert!(account.exists().await, "global contract account should exist after deployment");
}
