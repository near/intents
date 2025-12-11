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

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;
use defuse_transfer_auth::ext::DefuseAccountExt;
use defuse_nep413::SignedNep413Payload;



use defuse_transfer_auth::storage::StateInit as TransferAuthStateInit;

use anyhow::Result;
use defuse_core::amounts::Amounts;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::TransferMessage as ProxyTransferMessage;
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_escrow_swap::ext::derive_escrow_swap_account_id;
use defuse_escrow_swap::ContractStorage as EscrowContractStorage;
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
const PROXY: &str = "escrowproxy.pityjllk.testnet";

/// Derive a new ED25519 secret key from an account ID and derivation path
/// using a deterministic derivation based on the account ID and derivation info
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

    // Create new ED25519 secret key from the derived 32 bytes
    SecretKey::ED25519(ED25519SecretKey::from_secret_key(derived_bytes))
}

/// Create a subaccount with a deterministically derived key from the parent account ID.
/// Returns the `SigningAccount` for the new subaccount.
fn create_subaccount_with_derived_key(root: &SigningAccount, prefix: &str) -> Result<SigningAccount> {
    let random_suffix: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let subaccount_name = format!("{}-{}", prefix, random_suffix.to_lowercase());
    let derived_secret = derive_secret_key(root.id(), &subaccount_name);
    let derived_signer = Signer::from_secret_key(derived_secret)?;

    let subaccount = root.subaccount(&subaccount_name);
    Ok(SigningAccount::new(subaccount, derived_signer))
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
    subaccount.defuse_add_public_key(defuse, defuse_pubkey.clone()).await?;

    // 3. Verify registration by querying has_public_key
    let has_key: bool = defuse
        .call_function_json(
            "has_public_key",
            serde_json::json!({
                "account_id": subaccount.id(),
                "public_key": defuse_pubkey,
            }),
        )
        .await?;
    assert!(has_key, "Public key registration failed for {}", subaccount.id());
    println!("  Verified: {} public key registered in verifier", subaccount.id());

    Ok(())
}

/// Sign a transfer intent and return the signed NEP-413 payload
fn sign_transfer_intent(
    signer_id: &AccountId,
    secret_key: &[u8; 32],
    defuse_contract_id: &AccountId,
    transfer: Transfer,
    nonce: [u8; 32],
) -> SignedNep413Payload {
    use defuse_core::intents::{DefuseIntents, Intent};
    use defuse_core::payload::nep413::Nep413DefuseMessage;
    use defuse_crypto::Payload;
    use defuse_nep413::{Nep413Payload, SignedNep413Payload};
    use defuse_transfer_auth::ext::sign_ed25519;

    let deadline = Deadline::timeout(Duration::from_secs(300)); // 5 min

    let nep413_message = Nep413DefuseMessage {
        signer_id: signer_id.clone(),
        deadline,
        message: DefuseIntents {
            intents: vec![Intent::Transfer(transfer)],
        },
    };

    let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
        .with_recipient(defuse_contract_id)
        .with_nonce(nonce);

    let hash = nep413_payload.hash();
    let (public_key, signature) = sign_ed25519(secret_key, &hash);

    SignedNep413Payload {
        payload: nep413_payload,
        public_key,
        signature,
    }
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
    let maker_signing = create_subaccount_with_derived_key(&root, "maker")?;
    let maker_pubkey = maker_signing.signer().get_public_key().await?;
    println!("Maker: {} (pubkey: {})", maker_signing.id(), maker_pubkey);
    let taker_signing = create_subaccount_with_derived_key(&root, "taker")?;
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

    // // NOTE: requires 20 NEAR deposit
    // let global = root.deploy_transfer_auth("NEW_UNIQUE_ID").await;
    // println!("Transfer-auth global deployed: {global}");

    // // NOTE: requires 50 NEAR deposit
    // let global = root.deploy_escrow_swap_global("escrowswap").await;
    // println!("escrow-swap global deployed: {global}");

    // // 5. Create proxy subaccount and deploy escrow-proxy
    // println!("\n--- Deploying Proxy ---");
    // let proxy = root
    //     .create_subaccount("proxy", NearToken::from_yoctonear(1))
    //     .await?;
    // println!("Proxy account created: {}", proxy.id());

    // root.deploy_escrow_swap_global("escrow").await;

    // let initial_solver_balance =
    //     SigningAccount::mt_balance_of(&defuse, &solver.id(), &token_id)
    //         .await
    //         .unwrap();
    //
    // println!("Root account: {}", root.id());
    //
    // // 4. Deploy global contracts as subaccounts
    // println!("\n--- Deploying Global Contracts ---");
    // let escrow_global = root.deploy_escrow_swap_global("escrow").await;
    // println!("Escrow-swap global deployed: {escrow_global}");
    //
    // let auth_global = root.deploy_transfer_auth("auth").await;
    // println!("Transfer-auth global deployed: {auth_global}");
    //
    // // 5. Create proxy subaccount and deploy escrow-proxy
    // println!("\n--- Deploying Proxy ---");
    // let proxy = root
    //     .create_subaccount("proxy", NearToken::from_near(20))
    //     .await?;
    // println!("Proxy account created: {}", proxy.id());
    //
    // let defuse_id: AccountId = VERIFIER_CONTRACT.parse()?;
    // // Construct solverbus account ID as subaccount of root
    // let solverbus_id: AccountId = format!("solverbus.{}", root.id()).parse()?;

    // let roles = RolesConfig {
    //     super_admins: HashSet::from([root.id().clone()]),
    //     admins: std::collections::HashMap::new(),
    //     grantees: std::collections::HashMap::new(),
    // };
    //
    // let config = ProxyConfig {
    //     per_fill_contract_id: GlobalContractId::AccountId(TRANSFER_AUTH_GLOBAL_REF_ID.parse().unwrap()),
    //     escrow_swap_contract_id: GlobalContractId::AccountId(ESCROW_GLOBAL_REF_ID.parse().unwrap()),
    //     auth_contract: VERIFIER_CONTRACT.parse().unwrap(),
    //     auth_collee: root.id().clone(),
    // };

    // println!("{}", serde_json::to_string_pretty(&roles).unwrap());
    // println!("{}", serde_json::to_string_pretty(&config).unwrap());

    // proxy.deploy_escrow_proxy(roles, config.clone()).await?;
    // println!("Escrow-proxy deployed to: {}", proxy.id());
    //
    // // 6. Create solverbus and solver accounts
    // println!("\n--- Creating Solver Accounts ---");
    // let solverbus = root
    //     .create_subaccount("solverbus", NearToken::from_near(10))
    //     .await?;
    // println!("Solverbus account created: {}", solverbus.id());
    //
    // let solver = root
    //     .create_subaccount("solver", NearToken::from_near(10))
    //     .await?;
    // println!("Solver account created: {}", solver.id());
    //
    // // 7. Generate and register public keys in defuse
    // println!("\n--- Registering Public Keys ---");
    // let defuse = Account::new(defuse_id.clone(), network_config.clone());
    //
    // let solver_pubkey = public_key_from_secret(&SOLVER_SECRET);
    // solver
    //     .defuse_add_public_key(&defuse, solver_pubkey.clone())
    //     .await?;
    // println!("Solver public key registered: {solver_pubkey:?}");
    //
    // let solverbus_pubkey = public_key_from_secret(&SOLVERBUS_SECRET);
    // solverbus
    //     .defuse_add_public_key(&defuse, solverbus_pubkey.clone())
    //     .await?;
    // println!("Solverbus public key registered: {solverbus_pubkey:?}");
    //
    // 8. Create escrow params
    println!("\n--- Building Escrow Parameters ---");

    // For this demo, we'll use solver as both maker and taker to simplify
    // In real usage, maker would be a different account
    //TODO: genreate accounts on the fly
    // let maker_id = root.subaccount("maker");
    // let taker_id = root.subaccount("taker");

    let escrow_params = Params {
        maker: root.id().clone(),
        src_token: src_token.clone(),
        dst_token: dst_token.clone(),
        price: Price::ONE,
        deadline: Deadline::timeout(Duration::from_secs(300)), // 5 min
        partial_fills_allowed: false,
        refund_src_to: Default::default(),
        receive_dst_to: Default::default(),
        taker_whitelist: [PROXY.parse().unwrap()].into(),
        protocol_fees: None,
        integrator_fees: BTreeMap::new(),
        auth_caller: None,
        salt: rand::rng().random(),
    };

    let escrow_instance_id =
        derive_escrow_swap_account_id(&ESCROW_GLOBAL_REF_ID.parse().unwrap(), &escrow_params);
    println!("Derived escrow-swap instance ID: {escrow_instance_id}");

    let escrow_swap_msg = EscrowTransferMessage {
        params: escrow_params.clone(),
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: Deadline::timeout(Duration::from_secs(360)),
            receive_src_to: defuse_escrow_swap::OverrideSend::default()
                .receiver_id(taker_signing.id().clone()),
        }),
    };

    let proxy_msg = ProxyTransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: rand::rng().random(),
        msg: serde_json::to_string(&escrow_swap_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg)?;


    let transfer_auth_context = TransferAuthContext {
        sender_id: Cow::Borrowed(root.id().as_ref()),
        token_ids: Cow::Owned(vec![dst_token.to_string()]),
        amounts: Cow::Owned(vec![U128(1)]),
        salt: proxy_msg.salt,
        msg: Cow::Borrowed(&proxy_msg_json),
    };

    let transfer_auth_state_init = TransferAuthStateInit {
        escrow_contract_id: GlobalContractId::AccountId(ESCROW_GLOBAL_REF_ID.parse().unwrap()),
        auth_contract: VERIFIER_CONTRACT.parse().unwrap(),
        on_auth_signer: root.id().clone(),
        authorizee: PROXY.parse().unwrap(),
        msg_hash: transfer_auth_context.hash(),
    };

    // Create escrow state_init for deploying the escrow-swap instance
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(ESCROW_GLOBAL_REF_ID.parse().unwrap()),
        data: EscrowContractStorage::init_state(&escrow_params).unwrap(),
    });

    let transfer_intent = Transfer {
        receiver_id: proxy,
        tokens: Amounts::new([(dst_token.clone(), 1)].into()),
        memo: None,
        notification: Some(
            NotifyOnTransfer::new(proxy_msg_json.clone())
                .with_state_init(escrow_state_init)
        ),
    };


    // println!("\n--- Funding and Registering Subaccounts ---");
    // fund_and_register_subaccount(&root, &maker_signing, &defuse).await?;
    // fund_and_register_subaccount(&root, &taker_signing, &defuse).await?;


    // NOTE: required
    // root.tx(taker_account.id().clone())
    //     .create_account()
    //     .transfer(NearToken::from_near(5))
    //     .add_full_access_key(derived_pubkey.clone())
    //     .await?;
    //

    // let transfer_auth_instance_id =
    //     derive_transfer_auth_account_id(&ESCROW_GLOBAL_REF_ID.parse().unwrap());
    //
    // let transfer_payload = sign_transfer_intent(
    //     solver.id(),
    //     &SOLVER_SECRET,
    //     &defuse_id,
    //     transfer_intent,
    //     [10u8; 32], // nonce
    // );
    // println!("Transfer intent signed by solver");
    //
    // // 11. Build auth_call intent (solverbus authorizes)
    // println!("\n--- Building AuthCall Intent ---");
    //
    // // Compute context hash for transfer-auth
    // let context_hash = TransferAuthContext {
    //     sender_id: Cow::Borrowed(solver.id().as_ref()),
    //     token_ids: Cow::Owned(vec![token_b_defuse.to_string()]),
    //     amounts: Cow::Owned(vec![U128(SWAP_AMOUNT)]),
    //     msg: Cow::Borrowed(&proxy_msg_json),
    // }
    // .hash();
    //
    // let auth_state = TransferAuthState {
    //     escrow_contract_id: config.escrow_swap_contract_id.clone(),
    //     auth_contract: defuse_id.clone(),
    //     on_auth_signer: solverbus.id().clone(),
    //     authorizee: proxy.id().clone(),
    //     msg_hash: context_hash,
    // };
    //

    // let auth_payload = SigningAccount::sign_auth_call_intent(
    //     root.id(),
    //     &SOLVERBUS_SECRET,
    //     &defuse_id,
    //     &auth_global,
    //     &auth_state,
    //     [20u8; 32], // nonce
    // );

    // let transfer_auth_instance_id = derive_transfer_auth_account_id(
    //     &GlobalContractId::AccountId(auth_global.clone()),
    //     &auth_state,
    // );
    // println!("Transfer-auth instance ID: {transfer_auth_instance_id}");
    // println!("AuthCall intent signed by solverbus");
    //
    // // 12. Create combined execute_intents payloads
    // println!("\n--- Combined Intents for execute_intents ---");
    // let multi_payloads = vec![
    //     MultiPayload::Nep413(transfer_payload),
    //     MultiPayload::Nep413(auth_payload),
    // ];
    //
    // println!("Number of intents: {}", multi_payloads.len());
    // println!("Intent 0: Transfer (solver sends tokens to proxy)");
    // println!("Intent 1: AuthCall (solverbus authorizes the fill)");
    //
    // // 13. Print summary (don't execute)
    // println!("\n=== Summary ===");
    // println!("This demo prepared the following for testnet execution:");
    // println!("  - Global escrow-swap contract: {escrow_global}");
    // println!("  - Global transfer-auth contract: {auth_global}");
    // println!("  - Proxy contract: {}", proxy.id());
    // println!("  - Solver account: {}", solver.id());
    // println!("  - Solverbus account: {}", solverbus.id());
    // println!("  - Escrow instance (to be deployed): {escrow_instance_id}");
    // println!("  - Transfer-auth instance (to be deployed): {transfer_auth_instance_id}");
    // println!();
    // println!("To execute, call defuse.execute_intents with the signed payloads.");
    // println!("Note: In production, maker would first fund the escrow with src_token.");
    //
    Ok(())
}
