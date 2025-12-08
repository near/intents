mod env;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use defuse_deadline::Deadline;
use defuse_escrow_proxy::{EscrowParams, FillAction, ProxyConfig, RolesConfig, TransferMessage};
use defuse_sandbox::{Sandbox, SigningAccount};
use defuse_token_id::{TokenId, nep141::Nep141TokenId};
use defuse_transfer_auth::ext::{DefuseAccountExt, TransferAuthAccountExt};
use env::AccountExt;
use multi_token_receiver_stub::ext::MtReceiverStubAccountExt;
use near_sdk::{NearToken, json_types::U128};

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
    let solver = root.create_subaccount("solver", INIT_BALANCE).await.unwrap();

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    let config = ProxyConfig {
        per_fill_global_contract_id: transfer_auth_global.clone(),
        //NOTE: not really used in this test YET
        escrow_swap_global_contract_id: mt_receiver_global.clone(),
        auth_contract: defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };

    let proxy = sandbox
        .root()
        .create_subaccount("proxy", INIT_BALANCE)
        .await
        .unwrap();
    proxy.deploy_escrow_proxy(roles, config).await.unwrap();

    // === Test WNEAR deposit to defuse with solver as receiver ===

    // 1. Root deposits NEAR to get WNEAR
    let deposit_amount = NearToken::from_near(10);
    root.near_deposit(&wnear, deposit_amount).await.unwrap();

    // 2. Storage deposit for defuse contract on wnear (so defuse can receive tokens)
    root.ft_storage_deposit(&wnear, Some(defuse.id()))
        .await
        .unwrap();

    // 3. ft_transfer_call WNEAR to defuse with solver as msg (receiver)
    let transfer_amount: u128 = NearToken::from_near(5).as_yoctonear();
    root.ft_transfer_call(&wnear, defuse.id(), transfer_amount, solver.id().as_str())
        .await
        .unwrap();

    // 4. Query mt_balance_of for solver on defuse
    // Token ID for NEP-141 is "nep141:<contract_id>"
    let token_id = TokenId::from(Nep141TokenId::new(wnear.id().clone()));
    let balance = SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id.to_string())
        .await
        .unwrap();

    // 5. Assert balance > 0 and equals the transferred amount
    assert!(balance > 0, "Solver balance should be > 0, got {balance}");
    assert_eq!(
        balance, transfer_amount,
        "Balance should equal transferred amount"
    );

    // === Test MT transfer to proxy with timeout/refund ===

    // Record initial solver balance
    let initial_solver_balance =
        SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id.to_string())
            .await
            .unwrap();

    // Build TransferMessage for the proxy
    let transfer_msg = TransferMessage {
        fill_action: FillAction {
            price: 1,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
        },
        escrow_params: EscrowParams {
            maker: solver.id().clone(),
            src_token: token_id.clone(),
            dst_token: token_id.clone(),
            price: U128(1),
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: BTreeSet::new(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            salt: [0u8; 32],
        },
        salt: [1u8; 32],
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    // Transfer MT tokens from solver to proxy
    // The proxy will attempt to do state_init for transfer-auth and call wait_for_authorization
    // Currently the proxy has a bug with promise chaining, so the call will fail
    // but tokens should still be refunded to solver
    let proxy_transfer_amount = NearToken::from_near(1).as_yoctonear();
    let used_amounts = solver
        .mt_transfer_call(
            &defuse,
            &proxy.id(),
            &token_id.to_string(),
            proxy_transfer_amount,
            &msg_json,
        )
        .await
        .unwrap();

    // mt_transfer_call returns amounts that were "used" (not refunded)
    // When receiver fails/refunds, the used amount should be 0
    assert_eq!(
        used_amounts,
        vec![0],
        "Used amount should be 0 when transfer fails/refunds"
    );

    // Verify solver balance is unchanged after refund
    let final_solver_balance =
        SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id.to_string())
            .await
            .unwrap();

    assert_eq!(
        initial_solver_balance, final_solver_balance,
        "Solver balance should be unchanged after refund"
    );
}
