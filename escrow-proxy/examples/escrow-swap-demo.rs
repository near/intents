//! Example demonstrating escrow swap flow with proxy authorization on NEAR testnet.
//!
//! This example shows how to:
//! 1. Connect to NEAR testnet using environment credentials
//! 2. Deploy global escrow-swap and transfer-auth contracts
//! 3. Deploy escrow-proxy contract
//! 4. Create solver and solverbus accounts
//! 5. Build and sign transfer + auth_call intents for execute_intents
//!
//! Usage:
//!   USER=your-account.testnet PKEY=ed25519:... cargo run --example escrow-swap-demo --features test-utils
//!
//! Note: This example only builds the intents without executing them on testnet.

use defuse_core::intents::Intent;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_transfer_auth::ext::DefuseAccountExt;
use defuse_transfer_auth::ext::sign_intents;
use defuse_transfer_auth::storage::StateInit as TransferAuthStateInit;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use defuse_core::amounts::Amounts;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::TransferMessage as ProxyTransferMessage;
use defuse_escrow_swap::ContractStorage as EscrowContractStorage;
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_price::Price;
use defuse_sandbox::api::{NetworkConfig, SecretKey, Signer};
use defuse_sandbox::{Account, SigningAccount};
use defuse_token_id::TokenId;
use defuse_token_id::nep141::Nep141TokenId;
use defuse_transfer_auth::TransferAuthContext;
use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{AccountId, GlobalContractId, NearToken};
use rand::{Rng, distr::Alphanumeric};

// Placeholder constants - replace with actual testnet values
const VERIFIER_CONTRACT: &str = "intents.nearseny.testnet";

const SRC_NEP245_TOKEN_ID: &str = "src-token.omft.nearseny.testnet";
const DST_NEP245_TOKEN_ID: &str = "dst-token.omft.nearseny.testnet";
const DEFUSE_INSTANCE: &str = "intents.nearseny.testnet";

const TRANSFER_AUTH_GLOBAL_REF_ID: &str = "test2.pityjllk.testnet";
const ESCROW_GLOBAL_REF_ID: &str = "escrowswap.pityjllk.testnet";

// NOTE: 
// near contract deploy escrowproxy.pityjllk.testnet use-file /Users/mat/intents/res/defuse_escrow_proxy.wasm with-init-call new json-args '{"roles":{"super_admins":["pityjllk.testnet"],"admins":{},"grantees":{}},"config":{"per_fill_contract_id":"test2.pityjllk.testnet","escrow_swap_contract_id":"escrowswap.pityjllk.testnet","auth_contract":"intents.nearseny.testnet","auth_collee":"pityjllk.testnet"}}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-keychain send
const PROXY: &str = "escrowproxy.pityjllk.testnet";

/// Extract raw 32-byte secret from a SecretKey
fn extract_secret_bytes(secret_key: &SecretKey) -> [u8; 32] {
    match secret_key {
        SecretKey::ED25519(_ed_key) => {
            // The string format is "ed25519:<base58_encoded_secret>"
            // We need to parse and extract the secret portion
            let key_str = secret_key.to_string();
            let encoded = key_str.strip_prefix("ed25519:").unwrap();
            // Use near_sdk's bs58 re-export
            let decoded = near_sdk::bs58::decode(encoded).into_vec().unwrap();
            // First 32 bytes are the secret key
            decoded[..32].try_into().unwrap()
        }
        _ => panic!("Only ED25519 keys supported"),
    }
}

/// Derive a new ED25519 secret key from an account ID and derivation path
/// using a deterministic derivation based on the account ID and derivation info.
/// Returns (SecretKey for near-api, raw 32-byte secret for signing intents)
fn derive_secret_key(account_id: &AccountId, derivation_info: &str) -> (SecretKey, [u8; 32]) {
    use defuse_sandbox::api::{CryptoHash, types::crypto::secret_key::ED25519SecretKey};

    // Hash account ID with derivation info to create deterministic seed
    let mut seed_data = Vec::new();
    seed_data.extend_from_slice(account_id.as_str().as_bytes());
    seed_data.extend_from_slice(b":");
    seed_data.extend_from_slice(derivation_info.as_bytes());

    // Use CryptoHash to create a deterministic 32-byte seed
    let hash = CryptoHash::hash(&seed_data);
    let derived_bytes: [u8; 32] = hash.0;

    // Create new ED25519 secret key from the derived 32 bytes
    (
        SecretKey::ED25519(ED25519SecretKey::from_secret_key(derived_bytes)),
        derived_bytes,
    )
}

/// Create a subaccount with a deterministically derived key from the parent account ID.
/// Returns the `SigningAccount` for the new subaccount and the raw 32-byte secret key for signing.
fn create_subaccount_with_derived_key(
    root: &SigningAccount,
    prefix: &str,
) -> Result<(SigningAccount, [u8; 32])> {
    let random_suffix: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let subaccount_name = format!("{}-{}", prefix, random_suffix.to_lowercase());
    let (derived_secret, secret_bytes) = derive_secret_key(root.id(), &subaccount_name);
    let derived_signer = Signer::from_secret_key(derived_secret)?;

    let subaccount = root.subaccount(&subaccount_name);
    Ok((
        SigningAccount::new(subaccount, derived_signer),
        secret_bytes,
    ))
}

/// Fund a subaccount on-chain, register its public key in the verifier contract,
/// and verify the registration succeeded.
///
/// This function:
/// 1. Creates the account on-chain with 0.1 NEAR and the derived public key
/// 2. Registers the derived public key in the verifier (defuse) contract
/// 3. Queries the verifier to confirm the public key was registered correctly
async fn fund_and_register_subaccount(
    root: &SigningAccount,
    subaccount: &SigningAccount,
    defuse: &Account,
) -> Result<()> {
    let pubkey = subaccount.signer().get_public_key().await?;

    // 1. Create the account on-chain with 0.1 NEAR and the derived public key
    root.tx(subaccount.id().clone())
        .create_account()
        .transfer(NearToken::from_millinear(100)) // 0.1 NEAR
        .add_full_access_key(pubkey.clone())
        .await?;

    // 2. Register the public key in the verifier contract
    let defuse_pubkey: defuse_crypto::PublicKey = pubkey.clone().into();
    subaccount
        .defuse_add_public_key(defuse, defuse_pubkey.clone())
        .await?;

    // 3. Verify registration by querying has_public_key
    let has_key =
        SigningAccount::defuse_has_public_key(defuse, subaccount.id(), &defuse_pubkey).await?;
    assert!(
        has_key,
        "Public key registration failed for {}",
        subaccount.id()
    );
    println!(
        "  Verified: {} public key registered in verifier",
        subaccount.id()
    );

    Ok(())
}

/// Register account's public key in defuse if not already registered.
async fn register_root_pkey_in_defuse(account: &SigningAccount, defuse: &Account) -> Result<()> {
    let pubkey = account.signer().get_public_key().await?;
    let defuse_pubkey: defuse_crypto::PublicKey = pubkey.clone().into();
    let has_key = SigningAccount::defuse_has_public_key(defuse, account.id(), &defuse_pubkey).await?;
    if has_key {
        println!("{} public key already registered in defuse", account.id());
    } else {
        println!("{} public key NOT registered - registering...", account.id());
        account.defuse_add_public_key(defuse, defuse_pubkey).await?;
        println!("{} public key registered", account.id());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Escrow Swap Demo (Testnet) ===\n");

    // 1. Read environment variables
    let user = std::env::var("USER").map_err(|_| anyhow::anyhow!("USER env var not set"))?;
    let pkey = std::env::var("PKEY").map_err(|_| anyhow::anyhow!("PKEY env var not set"))?;

    println!("Using account: {user}");
    let network_config = NetworkConfig::testnet();
    println!("Network: testnet");

    // 3. Create SigningAccount from credentials
    let secret_key: SecretKey = pkey.parse()?;
    let signer = Signer::from_secret_key(secret_key.clone())?;
    let root = SigningAccount::new(Account::new(user.parse()?, network_config.clone()), signer);
    let proxy: AccountId = PROXY.parse().unwrap();

    let src_token: TokenId = Nep141TokenId::from_str(SRC_NEP245_TOKEN_ID).unwrap().into();
    let dst_token: TokenId = Nep141TokenId::from_str(DST_NEP245_TOKEN_ID).unwrap().into();
    let defuse = Account::new(DEFUSE_INSTANCE.parse()?, network_config.clone());

    // 4. Derive subaccount keys (deterministic, no network calls)
    println!("\n--- Deriving Subaccount Keys ---");
    let (maker_signing, maker_secret) = create_subaccount_with_derived_key(&root, "maker")?;
    let maker_pubkey = maker_signing.signer().get_public_key().await?;
    println!("Maker: {} (pubkey: {})", maker_signing.id(), maker_pubkey);
    let (taker_signing, taker_secret) = create_subaccount_with_derived_key(&root, "taker")?;
    let taker_pubkey = taker_signing.signer().get_public_key().await?;
    println!("Taker: {} (pubkey: {})", taker_signing.id(), taker_pubkey);

    // 5. Create accounts on-chain, fund them, and register public keys in verifier
    let (src_token_balance, dst_token_balance) = futures::try_join!(
        SigningAccount::mt_balance_of(&defuse, &root.id(), &src_token),
        SigningAccount::mt_balance_of(&defuse, &root.id(), &dst_token)
    )
    .unwrap();

    assert!(src_token_balance > 0);
    assert!(dst_token_balance > 0);

    println!("source token       ( {src_token} ) : {src_token_balance}");
    println!("destination token  ( {dst_token} ) : {dst_token_balance}");

    let deadline = Deadline::timeout(Duration::from_secs(300));
    // ESCROW SWAP PARAMS
    let escrow_params = Params {
        maker: maker_signing.id().clone(),
        src_token: Nep245TokenId::new(VERIFIER_CONTRACT.parse().unwrap(), src_token.to_string() ).into(),
        dst_token: Nep245TokenId::new(VERIFIER_CONTRACT.parse().unwrap(), dst_token.to_string() ).into(),
        price: Price::ONE,
        deadline: deadline, // 5 min
        partial_fills_allowed: false,
        refund_src_to: Default::default(),
        receive_dst_to: Default::default(),
        taker_whitelist: [proxy.clone()].into(),
        protocol_fees: None,
        integrator_fees: BTreeMap::new(),
        auth_caller: None,
        salt: rand::rng().random(),
    };
    // Build state_init for deploying escrow-swap instance
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(ESCROW_GLOBAL_REF_ID.parse().unwrap()),
        data: EscrowContractStorage::init_state(&escrow_params).unwrap(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();
    println!("Escrow-swap instance ID: {escrow_instance_id}");
    let escrow_fund_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fund,
    };
    let maker_transfer_intent = Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: Amounts::new([(src_token.clone(), 1)].into()),
        memo: None,
        notification: Some(
            NotifyOnTransfer::new(serde_json::to_string(&escrow_fund_msg).unwrap())
                .with_state_init(escrow_state_init.clone()),
        ),
    };

    // TAKER TRANSFER INETNT
    let escrow_fill_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: deadline,
            receive_src_to: defuse_escrow_swap::OverrideSend::default()
                .receiver_id(taker_signing.id().clone()),
        }),
    };
    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(), // escrow instance id
        salt: rand::rng().random(),
        msg: serde_json::to_string(&escrow_fill_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg)?;
    let taker_transfer_intent = Transfer {
        receiver_id: proxy,
        tokens: Amounts::new([(dst_token.clone(), 1)].into()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(proxy_msg_json.clone())),
    };

    // RELAY AUTH CALL INTENT
    // The relay authorizes the taker's transfer by signing an AuthCall intent
    // that deploys the transfer-auth instance with state matching the transfer context
    let transfer_auth_context = TransferAuthContext {
        sender_id: Cow::Borrowed(taker_signing.id().as_ref()),
        token_ids: Cow::Owned(vec![dst_token.to_string()]),
        amounts: Cow::Owned(vec![U128(1)]),
        salt: proxy_msg.salt,
        // NOTE: authorizes particular notification from taker(solver)
        msg: Cow::Borrowed(&proxy_msg_json),
    };

    // TransferAuthStateInit defines the state for the transfer-auth instance
    let transfer_auth_state = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(ESCROW_GLOBAL_REF_ID.parse().unwrap()),
        auth_contract: VERIFIER_CONTRACT.parse().unwrap(),
        on_auth_signer: root.id().clone(), // relay account that signs the auth
        authorizee: PROXY.parse().unwrap(),
        msg_hash: transfer_auth_context.hash(),
    };

    // Build state_init for deploying transfer-auth instance
    let transfer_auth_raw_state =
        defuse_transfer_auth::storage::ContractStorage::init_state(transfer_auth_state.clone())
            .unwrap();
    let transfer_auth_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(TRANSFER_AUTH_GLOBAL_REF_ID.parse().unwrap()),
        data: transfer_auth_raw_state,
    });
    let transfer_auth_instance_id = transfer_auth_state_init.derive_account_id();
    println!("Transfer-auth instance ID: {transfer_auth_instance_id}");

    // Create AuthCall intent that deploys transfer-auth and authorizes the transfer
    let relay_auth_call = defuse_core::intents::auth::AuthCall {
        contract_id: transfer_auth_instance_id.clone(),
        state_init: Some(transfer_auth_state_init),
        msg: String::new(),
        attached_deposit: NearToken::from_yoctonear(0),
        min_gas: None,
    };

    // ROOT TRANSFER INTENTS
    // Transfer src tokens from root to maker
    let root_to_maker_transfer = Transfer {
        receiver_id: maker_signing.id().clone(),
        tokens: Amounts::new([(src_token.clone(), 1)].into()),
        memo: None,
        notification: None,
    };

    // Transfer dst tokens from root to taker
    let root_to_taker_transfer = Transfer {
        receiver_id: taker_signing.id().clone(),
        tokens: Amounts::new([(dst_token.clone(), 1)].into()),
        memo: None,
        notification: None,
    };

    // === SIGN ALL INTENTS ===

    let defuse_id: &AccountId = defuse.id();
    let root_secret = extract_secret_bytes(&secret_key);

    println!("\n--- Signing Intents ---");

    let maker_sends_funds_to_escrow = sign_intents(
        maker_signing.id(),
        &maker_secret,
        defuse_id,
        rand::rng().random(),
        vec![Intent::Transfer(maker_transfer_intent)],
    );

    let taker_sends_funds_to_proxy = sign_intents(
        taker_signing.id(),
        &taker_secret,
        defuse_id,
        rand::rng().random(),
        vec![Intent::Transfer(taker_transfer_intent)],
    );

    let rooot_sends_funds_to_maker_and_taker = sign_intents(
        root.id(),
        &root_secret,
        defuse_id,
        rand::rng().random(),
        vec![
            Intent::Transfer(root_to_maker_transfer),
            Intent::Transfer(root_to_taker_transfer),
        ],
    );

    let root_sends_auth_call = sign_intents(
        root.id(),
        &root_secret,
        defuse_id,
        rand::rng().random(),
        vec![Intent::AuthCall(relay_auth_call)],
    );
    // EXECUTION
    println!("\n--- Execution ---");

    // Setup: Register public keys in defuse for all signers
    register_root_pkey_in_defuse(&root, &defuse).await?;
    fund_and_register_subaccount(&root, &maker_signing, &defuse).await?;
    fund_and_register_subaccount(&root, &taker_signing, &defuse).await?;

    // Step 1: Root transfers funds to maker and taker
    println!("\nStep 1: Root sends funds to maker and taker...");
    root.execute_signed_intents(&defuse, &[rooot_sends_funds_to_maker_and_taker])
        .await?;
    println!("  Done: maker and taker funded");

    // Step 2: Maker sends funds to escrow (deploys escrow-swap instance)
    println!("\nStep 2: Maker sends funds to escrow (deploys escrow-swap)...");
    root.execute_signed_intents(&defuse, &[maker_sends_funds_to_escrow])
        .await?;
    println!("  Done: escrow-swap instance deployed at {escrow_instance_id}");

    // Step 3: Root sends auth_call + taker sends funds to proxy (atomically)
    // This deploys transfer-auth instance and executes the fill through the proxy
    println!("\nStep 3: Root sends auth_call + taker sends funds to proxy...");
    root.execute_signed_intents(&defuse, &[root_sends_auth_call, taker_sends_funds_to_proxy])
        .await?;
    println!("  Done: transfer-auth deployed at {transfer_auth_instance_id}");
    println!("  Done: taker filled the escrow through proxy");


    println!("\n=== Escrow Swap Demo Complete ===");

    // Query escrow-swap instance state
    let escrow_account = Account::new(escrow_instance_id.clone(), network_config.clone());
    let escrow_state: defuse_escrow_swap::Storage = escrow_account
        .call_function_json("escrow_view", serde_json::json!({}))
        .await?;
    println!("  Escrow state: {}", serde_json::to_string_pretty(&escrow_state).unwrap());


    // Query transfer-auth instance state
    let transfer_auth_account =
        Account::new(transfer_auth_instance_id.clone(), network_config.clone());
    let transfer_auth_state: defuse_transfer_auth::storage::ContractStorage = transfer_auth_account
        .call_function_json("view", serde_json::json!({}))
        .await?;
    println!("  Transfer-auth state: {}", serde_json::to_string_pretty(&transfer_auth_state).unwrap());


    Ok(())
}
