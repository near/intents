mod env;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use defuse_deadline::Deadline;
use defuse_escrow_proxy::{EscrowParams, FillAction, ProxyConfig, RolesConfig, TransferAction, TransferMessage};
use defuse_escrow_swap::action::TransferMessage as EscrowTransferMessage;
use defuse_escrow_swap::price::Price;
use defuse_sandbox::{Sandbox, SigningAccount};
use defuse_token_id::{TokenId, nep141::Nep141TokenId};
use defuse_transfer_auth::ext::{DefuseAccountExt, TransferAuthAccountExt, derive_transfer_auth_account_id};
use defuse_transfer_auth::storage::{ContractStorage, StateInit as TransferAuthStateInit};
use defuse_transfer_auth::TransferAuthContext;
use near_sdk::json_types::U128;
use near_sdk::{Gas, GlobalContractId, state_init::{StateInit, StateInitV1}};
use env::AccountExt;
use multi_token_receiver_stub::ext::MtReceiverStubAccountExt;
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
    let solver = root.create_subaccount("solver", INIT_BALANCE).await.unwrap();

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(transfer_auth_global.clone()),
        //NOTE: not really used in this test YET
        escrow_swap_contract_id: GlobalContractId::AccountId(mt_receiver_global.clone()),
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
    root.ft_storage_deposit(&wnear, Some(&defuse.id()))
        .await
        .unwrap();

    // 3. ft_transfer_call WNEAR to defuse with solver as msg (receiver)
    let transfer_amount: u128 = NearToken::from_near(5).as_yoctonear();
    root.ft_transfer_call(&wnear, &defuse.id(), transfer_amount, solver.id().as_str())
        .await
        .unwrap();

    // 4. Query mt_balance_of for solver on defuse
    // Token ID for NEP-141 is "nep141:<contract_id>"
    let token_id = TokenId::from(Nep141TokenId::new(wnear.id().clone()));
    let balance = SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id)
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
        SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id)
            .await
            .unwrap();

    // Build the inner escrow-swap TransferMessage
    let inner_msg = EscrowTransferMessage {
        params: EscrowParams {
            maker: solver.id().clone(),
            src_token: token_id.clone(),
            dst_token: token_id.clone(),
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: BTreeSet::new(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            auth_caller: None,
            salt: [0u8; 32],
        },
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            receive_src_to: Default::default(),
        }),
    };
    let inner_msg_json = serde_json::to_string(&inner_msg).unwrap();

    // Build TransferMessage for the proxy (wraps inner message)
    let transfer_msg = TransferMessage {
        receiver_id: mt_receiver_instance.clone(),
        salt: [1u8; 32],
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    // Transfer MT tokens from solver to proxy
    // Run transfer and fast_forward concurrently - fast_forward triggers yield promise timeout
    let proxy_transfer_amount = NearToken::from_near(1).as_yoctonear();
    let solver_id = solver.id().clone();
    let proxy_id = proxy.id().clone();

    let (transfer_result, _) = futures::join!(
        solver.mt_transfer_call(
            &defuse,
            &proxy_id,
            &token_id,
            proxy_transfer_amount,
            &msg_json,
        ),
        sandbox.fast_forward(250)
    );

    let used_amounts = transfer_result.unwrap();

    // mt_transfer_call returns amounts that were "used" (not refunded)
    // When receiver times out/refunds, the used amount should be 0
    assert_eq!(
        used_amounts,
        vec![0],
        "Used amount should be 0 when transfer times out and refunds"
    );

    // Verify solver balance is unchanged after refund
    let final_solver_balance =
        SigningAccount::mt_balance_of(&defuse, &solver_id, &token_id)
            .await
            .unwrap();

    assert_eq!(
        initial_solver_balance, final_solver_balance,
        "Solver balance should be unchanged after timeout refund"
    );
}

/// Test that transfer succeeds when relay authorizes via on_auth call
#[tokio::test]
async fn test_transfer_authorized_by_relay() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    let wnear = root.deploy_wnear("wnear").await;
    let (transfer_auth_global, defuse, mt_receiver_global) = futures::join!(
        root.deploy_transfer_auth("global_transfer_auth"),
        root.deploy_verifier("defuse", wnear.id().clone()),
        root.deploy_mt_receiver_stub_global("mt_receiver_global"),
    );

    let relay = root.create_subaccount("relay", INIT_BALANCE).await.unwrap();
    let solver = root.create_subaccount("solver", INIT_BALANCE).await.unwrap();

    let roles = RolesConfig {
        super_admins: HashSet::from([root.id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(transfer_auth_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(mt_receiver_global.clone()),
        auth_contract: defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };

    let proxy = sandbox
        .root()
        .create_subaccount("proxy", INIT_BALANCE)
        .await
        .unwrap();
    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();

    // Derive and pre-deploy the escrow instance (mt-receiver-stub)
    // NOTE: In production, proxy should deploy this via state_init in mt_transfer_call
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(mt_receiver_global.clone()),
        data: BTreeMap::new(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    // Deploy the escrow instance via state_init
    // NOTE: Ignore RPC parsing errors - the tx succeeds but RPC response parsing may fail
    println!("STATE INIT!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    let result = root.tx(escrow_instance_id.clone())
        .state_init(mt_receiver_global.clone(), BTreeMap::new())
        .transfer(NearToken::from_yoctonear(1))
        .await;
    println!("{result:?} INITIALIZED!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    // Setup: deposit WNEAR to defuse for solver
    let deposit_amount = NearToken::from_near(10);
    root.near_deposit(&wnear, deposit_amount).await.unwrap();
    root.ft_storage_deposit(&wnear, Some(&defuse.id())).await.unwrap();

    let transfer_amount: u128 = NearToken::from_near(5).as_yoctonear();
    root.ft_transfer_call(&wnear, &defuse.id(), transfer_amount, solver.id().as_str())
        .await
        .unwrap();

    let token_id = TokenId::from(Nep141TokenId::new(wnear.id().clone()));

    // Record initial solver balance
    let initial_solver_balance =
        SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id)
            .await
            .unwrap();

    // Build the inner escrow-swap TransferMessage
    let inner_msg = EscrowTransferMessage {
        params: EscrowParams {
            maker: solver.id().clone(),
            src_token: token_id.clone(),
            dst_token: token_id.clone(),
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: BTreeSet::new(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            auth_caller: None,
            salt: [0u8; 32],
        },
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            receive_src_to: Default::default(),
        }),
    };
    let inner_msg_json = serde_json::to_string(&inner_msg).unwrap();

    // Build TransferMessage for the proxy (wraps inner message)
    let transfer_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [2u8; 32], // Different salt from timeout test
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let proxy_transfer_amount = NearToken::from_near(1).as_yoctonear();

    // Derive the transfer-auth instance address (same logic as proxy uses)
    // The hash is computed from TransferAuthContext, not the message itself
    let context_hash = TransferAuthContext {
        sender_id: Cow::Borrowed(&solver.id()),
        token_ids: Cow::Owned(vec![token_id.to_string()]),
        amounts: Cow::Owned(vec![U128(proxy_transfer_amount)]),
        msg: Cow::Borrowed(&msg_json),
    }.hash();

    let auth_state = TransferAuthStateInit {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: config.auth_contract.clone(),
        on_auth_signer: config.auth_collee.clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let transfer_auth_instance_id = derive_transfer_auth_account_id(&GlobalContractId::AccountId(transfer_auth_global.clone()), &auth_state);

    let proxy_id = proxy.id().clone();
    let relay_id = relay.id().clone();

    // Build raw state for state_init (same as proxy does)
    let raw_state = ContractStorage::init_state(auth_state).unwrap();

    // Run transfer and on_auth call concurrently
    // Transfer starts the yield promise, on_auth authorizes it
    // TODO: Replace direct on_auth call with AuthCall intent through defuse
    let (transfer_result, _auth_result) = futures::join!(
        solver.mt_transfer_call(
            &defuse,
            &proxy_id,
            &token_id,
            proxy_transfer_amount,
            &msg_json,
        ),
        // Call on_auth from defuse (auth_contract) with relay as signer_id
        // Include state_init to deploy the transfer-auth instance if not already deployed
        async {
            defuse.tx(transfer_auth_instance_id.clone())
                .state_init(transfer_auth_global.clone(), raw_state)
                .function_call_json::<()>(
                    "on_auth",
                    serde_json::json!({
                        "signer_id": relay_id,
                        "msg": "",
                    }),
                    Gas::from_tgas(50),
                    NearToken::from_yoctonear(1),
                )
                .no_result()
                .await
        }
    );

    let used_amounts = transfer_result.unwrap();

    // When authorized, tokens should be forwarded (used_amount = transferred amount)
    assert_eq!(
        used_amounts,
        vec![proxy_transfer_amount],
        "Used amount should equal transferred amount when authorized"
    );

    // Verify solver balance decreased
    let final_solver_balance =
        SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id)
            .await
            .unwrap();

    assert_eq!(
        initial_solver_balance - proxy_transfer_amount,
        final_solver_balance,
        "Solver balance should decrease by transferred amount"
    );

    // Verify escrow instance received the tokens
    let escrow_balance =
        SigningAccount::mt_balance_of(&defuse, &escrow_instance_id, &token_id)
            .await
            .unwrap();

    assert_eq!(
        escrow_balance, proxy_transfer_amount,
        "Escrow instance should have received the transferred tokens"
    );
}
