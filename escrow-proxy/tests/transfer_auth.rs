mod env;

use defuse_sandbox::{Account, Sandbox};
use defuse_transfer_auth::ext::TransferAuthAccountExt;
use env::AccountExt;

// #[tokio::test]
// async fn test_deploy_transfer_auth_global_contract() {
//     let sandbox = Sandbox::new().await;
//
//     let global_contract_id = sandbox.root().deploy_transfer_auth("global_transfer_auth").await;
//
//     // let config = ProxyConfig {
//     //     per_fill_global_contract_id: root.subaccount("per_fill_global_contract_id").id().clone(),
//     //     escrow_swap_global_contract_id: root
//     //         .subaccount("escrow_swap_global_contract_id")
//     //         .id()
//     //         .clone(),
//     //     auth_contract: root.subaccount("auth_contract").id().clone(),
//     //     auth_collee: root.subaccount("auth_collee").id().clone(),
//     // };
//
//
//
//     let proxy = sandbox.root().deploy_escrow_proxy("proxy").await;
//
//     let account = Account::new(global_contract_id, sandbox.root().network_config().clone());
//     assert!(account.exists().await, "global contract account should exist after deployment");
// }
