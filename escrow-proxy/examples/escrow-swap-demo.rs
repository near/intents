//! Example demonstrating escrow swap flow with proxy authorization on NEAR testnet.
//!
//! This example shows how to:
//! 1. Connect to NEAR testnet using environment credentials
//! 2. Deploy global escrow-swap and oneshot-condvar contracts
//! 3. Deploy escrow-proxy contract
//! 4. Create solver and solverbus accounts
//! 5. Build and sign transfer + `auth_call` intents for `execute_intents`
//!
//! Usage:
//!   USER=your-account.testnet PKEY=ed25519:... cargo run --example escrow-swap-demo --features test-utils
//!
//! Note: This example builds and executes intents on testnet.

use defuse::sandbox_ext::signer::DefuseSigner;
use defuse_core::Nonce;
use defuse_core::intents::{DefuseIntents, Intent};
use defuse_core::payload::multi::MultiPayload;
use defuse_oneshot_condvar::storage::Config as CondVarConfig;
use defuse_token_id::nep245::Nep245TokenId;
use near_sdk::serde_json;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use defuse_core::Deadline;
use defuse_core::amounts::Amounts;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_escrow_proxy::CondVarContext;
use defuse_escrow_proxy::TransferMessage as ProxyTransferMessage;
use defuse_escrow_swap::ContractStorage as EscrowContractStorage;
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::api::{NetworkConfig, SecretKey, Signer};
use defuse_sandbox::tx::FnCallBuilder;
use defuse_sandbox::{Account, MtViewExt, SigningAccount};
use defuse_token_id::TokenId;
use defuse_token_id::nep141::Nep141TokenId;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{AccountId, Gas, GlobalContractId, NearToken};
use rand::{Rng, distr::Alphanumeric};

// Placeholder constants - replace with actual testnet values
const VERIFIER_CONTRACT: &str = "intents.nearseny.testnet";

const SRC_NEP245_TOKEN_ID: &str = "src-token.omft.nearseny.testnet";
const DST_NEP245_TOKEN_ID: &str = "dst-token.omft.nearseny.testnet";
const DEFUSE_INSTANCE: &str = "intents.nearseny.testnet";

const ONESHOT_CONDVAR_GLOBAL_REF_ID: &str = "test2.pityjllk.testnet";
const ESCROW_GLOBAL_REF_ID: &str = "escrowswap.pityjllk.testnet";

// NOTE:
// near contract deploy escrowproxy.pityjllk.testnet use-file /Users/mat/intents/res/defuse_escrow_proxy.wasm with-init-call new json-args '{"roles":{"super_admins":["pityjllk.testnet"],"admins":{},"grantees":{}},"config":{"oneshot_condvar_global_id":"test2.pityjllk.testnet","escrow_swap_contract_id":"escrowswap.pityjllk.testnet","auth_contract":"intents.nearseny.testnet","notifier":"pityjllk.testnet"}}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-keychain send
const PROXY: &str = "escrowproxy.pityjllk.testnet";

/// Derive a new ED25519 secret key from an account ID and derivation path
/// using a deterministic derivation based on the account ID and derivation info.
fn derive_secret_key(account_id: &AccountId, derivation_info: &str) -> SecretKey {
    use defuse_sandbox::api::{CryptoHash, types::crypto::secret_key::ED25519SecretKey};

    // Hash account ID with derivation info to create deterministic seed
    let mut seed_data = Vec::new();
    seed_data.extend_from_slice(account_id.as_str().as_bytes());
    seed_data.extend_from_slice(b":");
    seed_data.extend_from_slice(derivation_info.as_bytes());

    // Use CryptoHash to create a deterministic 32-byte seed
    let hash = CryptoHash::hash(&seed_data);
    let derived_bytes: [u8; 32] = hash.0;

    SecretKey::ED25519(ED25519SecretKey::from_secret_key(derived_bytes))
}

/// Create a subaccount with a deterministically derived key from the parent account ID.
fn create_subaccount_with_derived_key(
    root: &SigningAccount,
    prefix: &str,
) -> Result<SigningAccount> {
    let random_suffix: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let subaccount_name = format!("{}-{}", prefix, random_suffix.to_lowercase());
    let derived_secret = derive_secret_key(root.id(), &subaccount_name);
    let derived_signer = Signer::from_secret_key(derived_secret)?;

    let subaccount = root.sub_account(&subaccount_name).unwrap();
    Ok(SigningAccount::new(subaccount, derived_signer))
}

// === Inline helper functions (previously from DefuseAccountExt) ===

/// Add a public key to an account in the defuse contract.
async fn defuse_add_public_key(
    account: &SigningAccount,
    defuse: &Account,
    public_key: defuse_crypto::PublicKey,
) -> anyhow::Result<()> {
    account
        .tx(defuse.id().clone())
        .function_call(
            FnCallBuilder::new("add_public_key")
                .json_args(json!({ "public_key": public_key }))
                .with_gas(Gas::from_tgas(50)),
        )
        .await?;
    Ok(())
}

/// Check if an account has a specific public key registered in defuse.
async fn defuse_has_public_key(
    defuse: &Account,
    account_id: &AccountId,
    public_key: &defuse_crypto::PublicKey,
) -> anyhow::Result<bool> {
    defuse
        .call_view_function_json(
            "has_public_key",
            json!({
                "account_id": account_id,
                "public_key": public_key,
            }),
        )
        .await
}

/// Execute signed intents on the defuse contract.
async fn execute_signed_intents(
    account: &SigningAccount,
    defuse: &Account,
    payloads: &[MultiPayload],
) -> anyhow::Result<()> {
    // Note: RPC may return parsing error but the tx succeeds
    account
        .tx(defuse.id().clone())
        .function_call(
            FnCallBuilder::new("execute_intents")
                .json_args(json!({ "signed": payloads }))
                .with_gas(Gas::from_tgas(300)),
        )
        .await
        .unwrap();
    Ok(())
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
        .add_full_access_key(pubkey)
        .await?;

    // 2. Register the public key in the verifier contract
    let defuse_pubkey: defuse_crypto::PublicKey = pubkey.into();
    defuse_add_public_key(subaccount, defuse, defuse_pubkey).await?;

    // 3. Verify registration by querying has_public_key
    let has_key = defuse_has_public_key(defuse, subaccount.id(), &defuse_pubkey).await?;
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
    let defuse_pubkey: defuse_crypto::PublicKey = pubkey.into();
    let has_key = defuse_has_public_key(defuse, account.id(), &defuse_pubkey).await?;
    if has_key {
        println!("{} public key already registered in defuse", account.id());
    } else {
        println!(
            "{} public key NOT registered - registering...",
            account.id()
        );
        defuse_add_public_key(account, defuse, defuse_pubkey).await?;
        println!("{} public key registered", account.id());
    }
    Ok(())
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
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
    let root = SigningAccount::new(
        Account::new(user.parse::<AccountId>()?, network_config.clone()),
        signer,
    );
    let proxy: AccountId = PROXY.parse().unwrap();

    let src_token: TokenId = Nep141TokenId::from_str(SRC_NEP245_TOKEN_ID).unwrap().into();
    let dst_token: TokenId = Nep141TokenId::from_str(DST_NEP245_TOKEN_ID).unwrap().into();
    let defuse = Account::new(
        DEFUSE_INSTANCE.parse::<AccountId>()?,
        network_config.clone(),
    );

    // 4. Derive subaccount keys (deterministic, no network calls)
    println!("\n--- Deriving Subaccount Keys ---");
    let maker_signing = create_subaccount_with_derived_key(&root, "maker")?;
    let maker_pubkey = maker_signing.signer().get_public_key().await?;
    println!("Maker: {} (pubkey: {})", maker_signing.id(), maker_pubkey);
    let taker_signing = create_subaccount_with_derived_key(&root, "taker")?;
    let taker_pubkey = taker_signing.signer().get_public_key().await?;
    println!("Taker: {} (pubkey: {})", taker_signing.id(), taker_pubkey);

    // 5. Create accounts on-chain, fund them, and register public keys in verifier
    let src_token_str = src_token.to_string();
    let dst_token_str = dst_token.to_string();
    let (src_token_balance, dst_token_balance) = futures::try_join!(
        defuse.mt_balance_of(root.id(), &src_token_str),
        defuse.mt_balance_of(root.id(), &dst_token_str)
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
        src_token: Nep245TokenId::new(
            VERIFIER_CONTRACT.parse::<AccountId>().unwrap(),
            src_token.to_string(),
        )
        .into(),
        dst_token: Nep245TokenId::new(
            VERIFIER_CONTRACT.parse::<AccountId>().unwrap(),
            dst_token.to_string(),
        )
        .into(),
        price: UD128::ONE,
        deadline, // 5 min
        partial_fills_allowed: false,
        refund_src_to: defuse_escrow_swap::OverrideSend::default(),
        receive_dst_to: defuse_escrow_swap::OverrideSend::default(),
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
            price: UD128::ONE,
            deadline,
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
    // that deploys the oneshot-condvar instance with state matching the transfer context
    let condvar_context = CondVarContext {
        escrow_contract_id: Cow::Owned(GlobalContractId::AccountId(
            ESCROW_GLOBAL_REF_ID.parse().unwrap(),
        )),
        sender_id: Cow::Borrowed(taker_signing.id().as_ref()),
        token_ids: Cow::Owned(vec![dst_token.to_string()]),
        amounts: Cow::Owned(vec![U128(1)]),
        salt: proxy_msg.salt,
        // NOTE: authorizes particular notification from taker(solver)
        msg: Cow::Borrowed(&proxy_msg_json),
    };

    // CondVarConfig defines the state for the oneshot-condvar instance
    let condvar_state = CondVarConfig {
        auth_contract: VERIFIER_CONTRACT.parse().unwrap(),
        notifier_id: root.id().clone(), // relay account that signs the auth
        authorizee: PROXY.parse().unwrap(),
        salt: condvar_context.hash(),
    };

    // Build state_init for deploying oneshot-condvar instance
    let condvar_raw_state =
        defuse_oneshot_condvar::storage::ContractStorage::init_state(condvar_state.clone())
            .unwrap();
    let condvar_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(ONESHOT_CONDVAR_GLOBAL_REF_ID.parse().unwrap()),
        data: condvar_raw_state,
    });
    let condvar_instance_id = condvar_state_init.derive_account_id();
    println!("OneshotCondVar instance ID: {condvar_instance_id}");

    // Create AuthCall intent that deploys oneshot-condvar and authorizes the transfer
    let relay_auth_call = defuse_core::intents::auth::AuthCall {
        contract_id: condvar_instance_id.clone(),
        state_init: Some(condvar_state_init),
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

    println!("\n--- Signing Intents ---");

    let maker_sends_funds_to_escrow = maker_signing
        .sign_defuse_message(
            defuse.id(),
            Nonce::from(rand::rng().random::<[u8; 32]>()),
            deadline,
            DefuseIntents {
                intents: vec![Intent::Transfer(maker_transfer_intent)],
            },
        )
        .await;

    let taker_sends_funds_to_proxy = taker_signing
        .sign_defuse_message(
            defuse.id(),
            Nonce::from(rand::rng().random::<[u8; 32]>()),
            deadline,
            DefuseIntents {
                intents: vec![Intent::Transfer(taker_transfer_intent)],
            },
        )
        .await;

    let root_sends_funds_to_maker_and_taker = root
        .sign_defuse_message(
            defuse.id(),
            Nonce::from(rand::rng().random::<[u8; 32]>()),
            deadline,
            DefuseIntents {
                intents: vec![
                    Intent::Transfer(root_to_maker_transfer),
                    Intent::Transfer(root_to_taker_transfer),
                ],
            },
        )
        .await;

    let root_sends_auth_call = root
        .sign_defuse_message(
            defuse.id(),
            Nonce::from(rand::rng().random::<[u8; 32]>()),
            deadline,
            DefuseIntents {
                intents: vec![Intent::AuthCall(relay_auth_call)],
            },
        )
        .await;
    // EXECUTION
    println!("\n--- Execution ---");

    // Setup: Register public keys in defuse for all signers
    register_root_pkey_in_defuse(&root, &defuse).await?;
    fund_and_register_subaccount(&root, &maker_signing, &defuse).await?;
    fund_and_register_subaccount(&root, &taker_signing, &defuse).await?;

    // Step 1: Root transfers funds to maker and taker
    println!("\nStep 1: Root sends funds to maker and taker...");
    execute_signed_intents(&root, &defuse, &[root_sends_funds_to_maker_and_taker]).await?;
    println!("  Done: maker and taker funded");

    // Step 2: Maker sends funds to escrow (deploys escrow-swap instance)
    println!("\nStep 2: Maker sends funds to escrow (deploys escrow-swap)...");
    execute_signed_intents(&root, &defuse, &[maker_sends_funds_to_escrow]).await?;
    println!("  Done: escrow-swap instance deployed at {escrow_instance_id}");

    // Step 3: Root sends auth_call + taker sends funds to proxy (atomically)
    // This deploys transfer-auth instance and executes the fill through the proxy
    println!("\nStep 3: Root sends auth_call + taker sends funds to proxy...");
    execute_signed_intents(
        &root,
        &defuse,
        &[root_sends_auth_call, taker_sends_funds_to_proxy],
    )
    .await?;
    println!("  Done: oneshot-condvar deployed at {condvar_instance_id}");
    println!("  Done: taker filled the escrow through proxy");

    println!("\n=== Escrow Swap Demo Complete ===");

    // Query escrow-swap instance state
    let escrow_account = Account::new(escrow_instance_id.clone(), network_config.clone());
    let escrow_state: defuse_escrow_swap::Storage = escrow_account
        .call_view_function_json("escrow_view", serde_json::json!({}))
        .await?;
    println!(
        "  Escrow state: {}",
        serde_json::to_string_pretty(&escrow_state).unwrap()
    );

    // Query oneshot-condvar instance state
    let condvar_account = Account::new(condvar_instance_id.clone(), network_config.clone());
    let condvar_state: defuse_oneshot_condvar::storage::ContractStorage = condvar_account
        .call_view_function_json("view", serde_json::json!({}))
        .await?;
    println!(
        "  OneshotCondVar state: {}",
        serde_json::to_string_pretty(&condvar_state).unwrap()
    );

    Ok(())
}
