//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Duration;
use std::{fs, path::Path};

use defuse_deadline::Deadline;
use defuse_escrow_proxy::ext::EscrowProxyAccountExt;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::action::{FillAction, TransferAction, TransferMessage as EscrowTransferMessage};
use defuse_escrow_swap::ext::{EscrowSwapAccountExt, derive_escrow_swap_account_id};
use defuse_escrow_swap::Params;
use defuse_poa_factory::contract::Role;
use defuse_price::Price;
use defuse_sandbox::{Account, Sandbox, SigningAccount};
use defuse_token_id::nep141::Nep141TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_token_id::TokenId;
use defuse_transfer_auth::ext::{DefuseAccountExt, TransferAuthAccountExt, derive_transfer_auth_account_id, public_key_from_secret};
use defuse_transfer_auth::storage::StateInit as TransferAuthState;
use defuse_transfer_auth::TransferAuthContext;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, GlobalContractId, NearToken};
use serde_json::json;

const INIT_BALANCE: NearToken = NearToken::from_near(100);
const SWAP_AMOUNT:u128 = 1_000_000_000_000_000_000_000_000;

/// Hardcoded test private key for relay (32 bytes) - FOR TESTING ONLY
const PRIVATE_KEY_RELAY: [u8; 32] = [1u8; 32];
const PRIVATE_KEY_SOLVER: [u8; 32] = [2u8; 32];

// ============================================================================
// Helper functions for deploying tokens via poa_factory using near-sandbox
// ============================================================================

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
}

static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/defuse_poa_factory"));

/// Deploy a new FT token via poa_factory with initial balances for specified accounts.
///
/// This high-level function:
/// 1. Deploys the token contract via poa_factory
/// 2. Registers storage for all specified accounts
/// 3. Mints the specified amounts to each account
///
/// Returns the token's AccountId (e.g., "token-a.factory.near")
async fn deploy_ft_token(
    deployer: &SigningAccount,
    factory: &Account,
    token_name: &str,
    initial_balances: &[(&AccountId, u128)],
) -> AccountId {
    // 1. Deploy token
    deployer
        .tx(factory.id().clone())
        .function_call_json::<()>(
            "deploy_token",
            json!({
                "token": token_name,
                "metadata": serde_json::Value::Null,
            }),
            Gas::from_tgas(100),
            NearToken::from_near(4),
        )
        .no_result()
        .await
        .unwrap();

    let token_id: AccountId = format!("{token_name}.{}", factory.id()).parse().unwrap();

    // 2. Storage deposit and mint for each account
    for (account_id, amount) in initial_balances {
        // Storage deposit
        deployer
            .tx(token_id.clone())
            .function_call_json::<serde_json::Value>(
                "storage_deposit",
                json!({ "account_id": account_id }),
                Gas::from_tgas(50),
                NearToken::from_millinear(50),
            )
            .await
            .ok();

        // Mint tokens
        if *amount > 0 {
            deployer
                .tx(factory.id().clone())
                .function_call_json::<()>(
                    "ft_deposit",
                    json!({
                        "token": token_name,
                        "owner_id": account_id,
                        "amount": U128(*amount),
                        "msg": serde_json::Value::Null,
                        "memo": serde_json::Value::Null,
                    }),
                    Gas::from_tgas(50),
                    NearToken::from_millinear(4),
                )
                .no_result()
                .await
                .unwrap();
        }
    }

    token_id
}

/// Deploy poa_factory contract
async fn deploy_poa_factory(deployer: &SigningAccount, name: impl AsRef<str>) -> Account {
    let account = deployer.subaccount(name);

    deployer
        .tx(account.id().clone())
        .create_account()
        .transfer(NearToken::from_near(100))
        .deploy(POA_FACTORY_WASM.clone())
        .function_call_json::<()>(
            "new",
            json!({
                "super_admins": HashSet::from([deployer.id().clone()]),
                "admins": HashMap::from([
                    (Role::TokenDeployer, HashSet::from([deployer.id().clone()])),
                    (Role::TokenDepositer, HashSet::from([deployer.id().clone()])),
                ]),
                "grantees": HashMap::from([
                    (Role::TokenDeployer, HashSet::from([deployer.id().clone()])),
                    (Role::TokenDepositer, HashSet::from([deployer.id().clone()])),
                ]),
            }),
            Gas::from_tgas(50),
            NearToken::from_yoctonear(0),
        )
        .no_result()
        .await
        .unwrap();

    account
}

/// Storage deposit for FT token
async fn ft_storage_deposit(
    payer: &SigningAccount,
    token: &AccountId,
    account: &AccountId,
) {
    payer
        .tx(token.clone())
        .function_call_json::<serde_json::Value>(
            "storage_deposit",
            json!({ "account_id": account }),
            Gas::from_tgas(50),
            NearToken::from_millinear(50),
        )
        .await
        .ok();
}

/// Deploy a new FT token and wrap it into defuse MT with initial balances.
///
/// This high-level function:
/// 1. Deploys the FT token contract via poa_factory
/// 2. Registers storage for defuse and all specified owners
/// 3. Mints FT tokens to each owner
/// 4. Wraps (deposits) the FT tokens into defuse MT for each owner
///
/// Returns:
/// - FT AccountId (for storage deposits on the underlying FT contract)
/// - Defuse MT TokenId (Nep141 format: "nep141:token.factory" - for defuse balance queries)
/// - Escrow MT TokenId (Nep245 format: "nep245:defuse:nep141:token.factory" - for escrow params)
async fn deploy_mt_token(
    deployer: &SigningAccount,
    factory: &Account,
    defuse: &SigningAccount,
    token_name: &str,
    initial_balances: &[(&SigningAccount, u128)],
) -> (AccountId, TokenId, TokenId) {
    // 1. Deploy FT token with storage for defuse
    let ft_token_id = deploy_ft_token(
        deployer,
        factory,
        token_name,
        &[(defuse.id(), 0)], // defuse needs storage to receive FT deposits
    ).await;

    // 2. For each owner: storage deposit, mint, and wrap into MT
    for (owner, amount) in initial_balances {
        if *amount == 0 {
            continue;
        }

        // Storage deposit for owner on FT
        ft_storage_deposit(deployer, &ft_token_id, owner.id()).await;

        // Mint FT tokens to owner
        deployer
            .tx(factory.id().clone())
            .function_call_json::<()>(
                "ft_deposit",
                json!({
                    "token": token_name,
                    "owner_id": owner.id(),
                    "amount": U128(*amount),
                    "msg": serde_json::Value::Null,
                    "memo": serde_json::Value::Null,
                }),
                Gas::from_tgas(50),
                NearToken::from_millinear(4),
            )
            .no_result()
            .await
            .unwrap();

        // Wrap FT into MT by depositing to defuse
        owner
            .tx(ft_token_id.clone())
            .function_call_json::<U128>(
                "ft_transfer_call",
                json!({
                    "receiver_id": defuse.id(),
                    "amount": U128(*amount),
                    "msg": owner.id().to_string(),
                }),
                Gas::from_tgas(100),
                NearToken::from_yoctonear(1),
            )
            .await
            .unwrap();
    }

    // Defuse uses Nep141 format internally for token IDs
    let defuse_mt_token_id = TokenId::from(Nep141TokenId::new(ft_token_id.clone()));

    // Escrow-swap receives tokens via mt_on_transfer which constructs Nep245TokenId
    // format: nep245:<defuse_contract>:<inner_token_id>
    let inner_token_id = defuse_mt_token_id.to_string();
    let escrow_mt_token_id = TokenId::from(Nep245TokenId::new(defuse.id().clone(), inner_token_id));

    (ft_token_id, defuse_mt_token_id, escrow_mt_token_id)
}

/// Test full escrow swap flow with proxy authorization
#[tokio::test]
async fn test_escrow_swap_with_proxy_full_flow() {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();

    // 1. Deploy core infrastructure
    let wnear = root.deploy_wnear("wnear").await;
    let (transfer_auth_global, defuse, poa_factory) = futures::join!(
        root.deploy_transfer_auth("transfer_auth"),
        root.deploy_verifier("defuse", wnear.id().clone()),
        deploy_poa_factory(root, "factory"),
    );

    // Deploy escrow-swap global contract
    let escrow_swap_global = root.deploy_escrow_swap_global("escrow_swap").await;

    // Create accounts
    let (maker, solver, relay) = futures::join!(
        root.create_subaccount("maker", INIT_BALANCE),
        root.create_subaccount("solver", INIT_BALANCE),
        root.create_subaccount("relay", INIT_BALANCE),
    );
    let maker = maker.unwrap();
    let solver = solver.unwrap();
    let relay = relay.unwrap();

    // Deploy escrow-proxy
    let roles = RolesConfig {
        super_admins: HashSet::from([root.id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };
    let proxy = root.create_subaccount("proxy", INIT_BALANCE).await.unwrap();

    // 2. Deploy and setup tokens
    let swap_amount: u128 = 1_000_000_000_000_000_000_000_000; // 1 token with 24 decimals

    // Deploy token-a: maker gets initial balance wrapped into defuse MT
    // Returns: (ft_id, defuse_mt_id for queries, escrow_mt_id for escrow params)
    let (token_a_id, token_a_defuse_id, token_a_escrow_id) = deploy_mt_token(
        root,
        &poa_factory,
        &defuse,
        "token-a",
        &[(&maker, swap_amount)],
    ).await;

    // Deploy token-b: solver gets initial balance wrapped into defuse MT
    let (token_b_id, token_b_defuse_id, token_b_escrow_id) = deploy_mt_token(
        root,
        &poa_factory,
        &defuse,
        "token-b",
        &[(&solver, swap_amount)],
    ).await;



    /////TEST

    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(transfer_auth_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        auth_contract: defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };
    proxy.deploy_escrow_proxy(roles, config.clone()).await.unwrap();

    // 3. Create escrow parameters (use escrow token IDs - Nep245 format)
    let escrow_params = Params {
        maker: maker.id().clone(),
        src_token: token_a_escrow_id.clone(),  // What maker offers (Nep245 format)
        dst_token: token_b_escrow_id.clone(),  // What maker wants (Nep245 format)
        price: Price::ONE,                 // 1:1 exchange
        deadline: Deadline::timeout(Duration::from_secs(360)),
        partial_fills_allowed: false,
        refund_src_to: Default::default(),
        receive_dst_to: Default::default(),
        taker_whitelist: [proxy.id().clone()].into(),  // Anyone can fill
        protocol_fees: None,
        integrator_fees: BTreeMap::new(),
        auth_caller: None,
        salt: [1u8; 32],
    };

    // Derive escrow instance ID (without deploying) to check existence
    let escrow_instance_id = derive_escrow_swap_account_id(&escrow_swap_global, &escrow_params);
    let escrow_instance_account = Account::new(escrow_instance_id.clone(), root.network_config().clone());

    // Verify escrow-swap instance does NOT exist before maker's fund
    assert!(
        !escrow_instance_account.exists().await,
        "Escrow-swap instance should NOT exist before maker's fund"
    );

    // 4. Fund escrow via Transfer intent with state_init
    // This deploys the escrow instance atomically with the token transfer
    let fund_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fund,
    };
    let fund_msg_json = serde_json::to_string(&fund_msg).unwrap();

    // Build state_init for escrow-swap instance
    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = near_sdk::state_init::StateInit::V1(near_sdk::state_init::StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(escrow_swap_global.clone()),
        data: escrow_raw_state,
    });

    // Build Transfer intent with notification containing state_init
    let transfer = defuse_core::intents::tokens::Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: defuse_core::amounts::Amounts::new([(token_a_defuse_id.clone(), swap_amount)].into()),
        memo: None,
        notification: Some(defuse_core::intents::tokens::NotifyOnTransfer::new(fund_msg_json)
            .with_state_init(escrow_state_init)),
    };

    // Maker registers public key and executes transfer intent
    const PRIVATE_KEY_MAKER: [u8; 32] = [3u8; 32];
    maker.defuse_add_public_key(&*defuse, public_key_from_secret(&PRIVATE_KEY_MAKER)).await.unwrap();
    maker.execute_transfer_intent(&defuse, transfer, &PRIVATE_KEY_MAKER, [0u8; 32]).await.unwrap();

    // Verify escrow-swap instance EXISTS after maker's fund (state_init deployed it)
    assert!(
        escrow_instance_account.exists().await,
        "Escrow-swap instance should exist after maker's fund"
    );

    // Verify escrow received maker's tokens
    let escrow_token_a_balance = SigningAccount::mt_balance_of(&*defuse, &escrow_instance_id, &token_a_defuse_id)
        .await
        .unwrap();
    assert_eq!(escrow_token_a_balance, swap_amount, "Escrow should have token-a");

    // 5. Solver fills via proxy with relay authorization
    // Build the inner escrow-swap fill message
    // NOTE: receive_src_to must be set to solver's account because escrow-swap
    // will use sender_id (proxy) as default recipient. Since proxy forwards tokens,
    // we need to explicitly override where maker's src tokens go.
    let inner_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: Deadline::timeout(Duration::from_secs(120)),
            receive_src_to: defuse_escrow_swap::OverrideSend::default()
                .receiver_id(solver.id().clone()),
        }),
    };
    let inner_msg_json = serde_json::to_string(&inner_msg).unwrap();

    // Build proxy message that wraps the inner escrow message
    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: [2u8; 32],
        msg: inner_msg_json,
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    // Derive transfer-auth instance from context hash
    // The hash is computed from TransferAuthContext matching how proxy computes it
    let context_hash = TransferAuthContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        msg: Cow::Borrowed(&proxy_msg_json),
    }.hash();

    let auth_state = TransferAuthState {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: defuse.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let transfer_auth_instance_id = derive_transfer_auth_account_id(&GlobalContractId::AccountId(transfer_auth_global.clone()), &auth_state);

    // Create Account wrapper for transfer-auth instance to check existence
    let transfer_auth_instance_account = Account::new(
        transfer_auth_instance_id.clone(),
        root.network_config().clone(),
    );

    // Verify transfer-auth instance does NOT exist before on_auth call
    assert!(
        !transfer_auth_instance_account.exists().await,
        "Transfer-auth instance should NOT exist before on_auth call"
    );

    // Storage deposits for proxy to forward tokens
    ft_storage_deposit(root, &token_a_id, solver.id()).await;
    ft_storage_deposit(root, &token_b_id, maker.id()).await;

    // Step 1: Execute AuthCall intent signed by relay to deploy and authorize transfer-auth
    // The relay signs an AuthCall intent with state_init, defuse executes it and calls on_auth
    // Note: Relay must be registered in defuse with their public key first
    relay.defuse_add_public_key(&*defuse, public_key_from_secret(&PRIVATE_KEY_RELAY)).await.unwrap();
    relay.execute_auth_call_intent(&defuse, &transfer_auth_global, &auth_state, &PRIVATE_KEY_RELAY, [0u8; 32]).await;

    // Verify transfer-auth instance EXISTS after on_auth call (state_init deployed it)
    assert!(
        transfer_auth_instance_account.exists().await,
        "Transfer-auth instance should exist after on_auth call"
    );

    // Step 2: Solver sends tokens to proxy - proxy will query transfer-auth for authorization
    let transfer_result = solver.mt_transfer_call(
        &*defuse,
        proxy.id(),
        &token_b_defuse_id,
        swap_amount,
        &proxy_msg_json,
    ).await;

    let used_amounts = transfer_result.unwrap();

    // 6. Verify final state
    // When authorized, tokens should be forwarded and swap completed
    assert_eq!(
        used_amounts,
        vec![swap_amount],
        "Used amount should equal transferred amount when authorized"
    );

    // Maker should have received token-b
    let maker_token_b_balance = SigningAccount::mt_balance_of(&*defuse, maker.id(), &token_b_defuse_id)
        .await
        .unwrap();
    assert_eq!(
        maker_token_b_balance, swap_amount,
        "Maker should have received token-b"
    );

    // Solver should have received token-a
    let solver_token_a_balance = SigningAccount::mt_balance_of(&*defuse, solver.id(), &token_a_defuse_id)
        .await
        .unwrap();
    assert_eq!(
        solver_token_a_balance, swap_amount,
        "Solver should have received token-a"
    );

    // Escrow should be empty
    let escrow_token_a_final = SigningAccount::mt_balance_of(&*defuse, &escrow_instance_id, &token_a_defuse_id)
        .await
        .unwrap();
    assert_eq!(
        escrow_token_a_final, 0,
        "Escrow should have no token-a remaining"
    );

    let escrow_token_b_final = SigningAccount::mt_balance_of(&*defuse, &escrow_instance_id, &token_b_defuse_id)
        .await
        .unwrap();
    assert_eq!(
        escrow_token_b_final, 0,
        "Escrow should have no token-b remaining"
    );
}
