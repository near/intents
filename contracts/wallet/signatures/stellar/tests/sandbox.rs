#![cfg(feature = "signer")]

#[path = "../../../test-utils/entrypoints.rs"]
mod entrypoints;

use std::{env, fs, path::Path};

use defuse_nep641::resolver::RpcResolver;
use defuse_wallet::{
    AccountId, Gas, NearPromise, NearToken, Request, StateInit, StateInitV1, WalletOp,
    actions::FunctionCall,
};
use defuse_wallet_sdk::{Wallet, WalletBuilder, WalletSigner, client::WalletContract};
use defuse_wallet_stellar::{
    MAX_SIGNING_MESSAGE_BYTES, WalletStellar, WalletStellarSigner,
    crypto::ed25519::ed25519_dalek::SigningKey, request_signing_message,
};
use near_kit::{
    Near, protocol::PublishMode, rpc::FinalExecutionOutcome, signer::InMemorySigner,
    transaction::Final,
};
use near_kit_sandbox::SandboxConfig;
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use sha2::{Digest, Sha256};

async fn near() -> Near {
    if let Ok(url) = env::var("STELLAR_SANDBOX_RPC_URL") {
        // This override is strictly for an isolated native sandbox, never a public network.
        assert!(is_loopback_sandbox_url(&url));
        let signer = InMemorySigner::new("sandbox", "ed25519:3tgdk2wPraJzT4nsTuf86UX41xgPNk3MHnq8epARMdBNs29AFEztAuaQ7iHddDfXG9F2RzV1XNQYgJyAyoW51UBB").unwrap();
        let provisional = Near::custom(&url, "sandbox").build();
        let chain = provisional.rpc().status().await.unwrap().chain_id;
        Near::custom(url, chain).signer(signer).build()
    } else {
        SandboxConfig::shared()
            .await
            .expect("NEAR sandbox requires Docker or STELLAR_SANDBOX_RPC_URL")
            .client()
    }
}

fn is_loopback_sandbox_url(url: &str) -> bool {
    let Some(endpoint) = url
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| url.strip_prefix("http://localhost:"))
    else {
        return false;
    };
    let port = endpoint.strip_suffix('/').unwrap_or(endpoint);
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|port| port != 0)
}

#[test]
fn sandbox_url_guard_rejects_remote_hosts_and_credentials() {
    for url in ["http://127.0.0.1:49206", "http://localhost:49206/"] {
        assert!(is_loopback_sandbox_url(url));
    }
    for url in [
        "http://127.0.0.1:80@attacker.example/",
        "http://localhost:80@attacker.example/",
        "http://127.0.0.1.attacker.example:80/",
        "http://localhost:0",
        "http://localhost:65536",
        "http://localhost:+80",
        "https://localhost:80",
        "http://localhost:80/path",
        "http://localhost:80?redirect=attacker.example",
    ] {
        assert!(!is_loopback_sandbox_url(url), "accepted {url}");
    }
}

async fn execute(
    near: &Near,
    wallet: &Wallet<WalletStellar>,
    request: impl Into<Request>,
) -> FinalExecutionOutcome {
    let outcome = wallet
        .sign_and_send(request)
        .await
        .unwrap()
        .status(near)
        .wait_until::<Final>()
        .await
        .unwrap();
    outcome.result().unwrap();
    outcome
}

#[tokio::test]
async fn wallet_executes_and_resolves_authorization_in_wasm() {
    let near = near().await;
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .join(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("res"));
    let wasm = fs::read(directory.join("defuse-wallet-stellar.wasm"))
        .expect("build with make defuse-wallet-stellar first");
    let code_hash: [u8; 32] = Sha256::digest(&wasm).into();
    near.publish(wasm, PublishMode::Immutable)
        .await
        .unwrap()
        .result()
        .unwrap();
    let signer = WalletStellarSigner(SigningKey::generate(&mut UnwrapErr(SysRng)));
    let wallet = Wallet::<WalletStellar>::new(code_hash, signer.clone())
        .with_client(near.clone())
        .with_relayer(near.clone());
    let same_key_subwallet = WalletBuilder::new()
        .subwallet_id(1)
        .build(code_hash, signer.clone());
    assert_ne!(wallet.account_id(), same_key_subwallet.account_id());
    let recipient = WalletBuilder::new()
        .subwallet_id(1)
        .build(
            code_hash,
            WalletStellarSigner(SigningKey::generate(&mut UnwrapErr(SysRng))),
        )
        .with_client(near.clone())
        .with_relayer(near.clone());
    assert_eq!(
        wallet.deterministic_state_init().derive_account_id(),
        *wallet.account_id()
    );
    assert_ne!(wallet.account_id(), recipient.account_id());
    wallet.initialize().await.unwrap();
    recipient.initialize().await.unwrap();
    let contract = near.contract::<WalletContract>(wallet.account_id());
    assert_eq!(
        contract.w_public_key().await.unwrap(),
        wallet.public_key().to_string()
    );

    // Fund only these isolated sandbox accounts; this key is public test material.
    near.transaction(wallet.account_id())
        .transfer(NearToken::from_near(5))
        .send()
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();

    let (msg, proof) = wallet
        .sign(defuse_wallet::WalletOp::add_extension(
            near.account_id().clone(),
        ))
        .await
        .unwrap();
    entrypoints::check(&near, wallet.account_id(), &msg, &proof).await;
    let ft_wasm = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../releases/fungible-token.wasm"),
    )
    .unwrap();
    let failure = entrypoints::metadata(&near, wallet.account_id(), &ft_wasm).await;
    let (msg, proof) = wallet.sign(failure).await.unwrap();
    entrypoints::failed_metadata_consumes_nonce(&near, wallet.account_id(), &msg, &proof).await;
    let (msg, _) = wallet.sign(Request::new()).await.unwrap();
    entrypoints::context(&near, wallet.account_id(), &msg, &signer).await;
    entrypoints::weight_overflow(&near, wallet.account_id(), &msg, &signer).await;

    let before = near.balance(recipient.account_id()).await.unwrap().total;
    execute(
        &near,
        &wallet,
        NearPromise::new(recipient.account_id()).transfer(NearToken::from_near(1)),
    )
    .await;
    assert_eq!(
        near.balance(recipient.account_id()).await.unwrap().total,
        before.saturating_add(NearToken::from_near(1))
    );

    check_ft_metadata(&near, &wallet, &recipient).await;
    check_nonce_rollback_and_replay(&near, &wallet).await;
    check_largest_message(&near, &wallet, &recipient, &signer).await;

    // The full NEP-641 route resolves exactly the signed payload, including whitespace.
    let payload = "{ \"domain\": \"Stellar test\", \"amount\": \"9007199254740993\" }";
    let authorization = wallet.sign_offchain_msg(payload, None).await.unwrap();
    let resolver = RpcResolver::new(near.rpc().clone()).await.unwrap();
    assert_eq!(
        resolver
            .resolve_auth(wallet.account_id(), &authorization)
            .await
            .unwrap(),
        payload
    );

    check_lockout_and_rotation(&near, wallet, recipient, payload, resolver).await;
}

async fn check_nonce_rollback_and_replay(near: &Near, wallet: &Wallet<WalletStellar>) {
    let contract = near.contract::<WalletContract>(wallet.account_id());
    // A bad proof must roll back the nonce commitment; its valid counterpart still works.
    let (msg, proof) = wallet.sign(Request::new()).await.unwrap();
    // Malformed, and a valid signature of another message.
    let (_, other) = wallet.sign(Request::new()).await.unwrap();
    for invalid in ["ed25519:invalid", other.as_str()] {
        contract
            .w_execute_signed((&msg, invalid).into())
            .gas(Gas::from_tgas(30))
            .await
            .unwrap()
            .result()
            .expect_err("invalid proof must fail");
    }
    contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(30))
        .await
        .unwrap()
        .result()
        .unwrap();
    contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(30))
        .await
        .unwrap()
        .result()
        .expect_err("nonce replay must fail");
}

/// Gas of a message at the 16 KiB limit; a larger one is rejected without a
/// state change.
async fn check_largest_message(
    near: &Near,
    wallet: &Wallet<WalletStellar>,
    recipient: &Wallet<WalletStellar>,
    signer: &WalletStellarSigner<SigningKey>,
) {
    let contract = near.contract::<WalletContract>(wallet.account_id());
    let (original, _) = wallet.sign(Request::new()).await.unwrap();
    let with_padding = |length| {
        let mut msg = original.clone();
        msg.request = NearPromise::new(recipient.account_id())
            .function_call(
                FunctionCall::name("w_public_key")
                    .args(format!("{{\"padding\":\"{}\"}}", "a".repeat(length)))
                    .gas(Gas::from_tgas(5))
                    .unused_gas_weight(0),
            )
            .into();
        msg
    };
    let key = wallet.public_key();
    let fixed = request_signing_message(&key, &with_padding(0))
        .unwrap()
        .len();
    // Each ASCII padding byte adds exactly one signed byte.
    let msg = with_padding(MAX_SIGNING_MESSAGE_BYTES - fixed);
    let length = request_signing_message(&key, &msg).unwrap().len();
    assert_eq!(length, MAX_SIGNING_MESSAGE_BYTES);
    let oversized = with_padding(MAX_SIGNING_MESSAGE_BYTES - fixed + 1);
    assert!(request_signing_message(&key, &oversized).is_err());
    let storage = || async {
        near.account(wallet.account_id())
            .await
            .unwrap()
            .storage_usage
    };
    let before = storage().await;
    let (_, other) = wallet.sign(Request::new()).await.unwrap();
    let failed = contract
        .w_execute_signed((&oversized, other.as_str()).into())
        .gas(Gas::from_tgas(80))
        .await
        .unwrap();
    failed.result().expect_err("oversized message must fail");
    assert_eq!(storage().await, before);
    let proof = signer.sign_request_msg(&msg).await.unwrap();
    let result = contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(80))
        .await
        .unwrap();
    result.result().unwrap();
    println!(
        "STELLAR_RESOURCE_PROFILE message_bytes={length} proof_bytes={} gas={} oversized_rejection_gas={} storage_before={before} storage_after={}",
        proof.len(),
        result.total_gas_used().as_gas(),
        failed.total_gas_used().as_gas(),
        storage().await
    );
}

async fn check_lockout_and_rotation(
    near: &Near,
    wallet: Wallet<WalletStellar>,
    recipient: Wallet<WalletStellar>,
    payload: &str,
    resolver: RpcResolver,
) {
    let contract = near.contract::<WalletContract>(wallet.account_id());
    // Lockout and external self-call checks remain enforced by the shared core.
    let (msg, proof) = wallet.sign(WalletOp::disable_signature()).await.unwrap();
    contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(30))
        .await
        .unwrap()
        .result()
        .expect_err("last authentication method cannot be disabled");
    let (msg, proof) = wallet
        .sign(NearPromise::new(wallet.account_id()).transfer(NearToken::from_yoctonear(1)))
        .await
        .unwrap();
    contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(30))
        .await
        .unwrap()
        .result()
        .expect_err("self-call must fail");

    // Installing another Stellar wallet as an extension and rotating control works.
    execute(
        near,
        &wallet,
        WalletOp::add_extension(recipient.account_id().clone()),
    )
    .await;
    execute(near, &wallet, WalletOp::disable_signature()).await;
    let (msg, proof) = wallet.sign(Request::new()).await.unwrap();
    contract
        .w_execute_signed((&msg, proof.as_str()).into())
        .gas(Gas::from_tgas(30))
        .await
        .unwrap()
        .result()
        .expect_err("disabled signatures must fail");
    let extension = recipient.as_extension_of(wallet);
    execute(near, &extension, Request::new()).await;
    let authorization = extension.sign_offchain_msg(payload, None).await.unwrap();
    assert_eq!(
        resolver
            .with_max_sub_authorizations(1)
            .with_max_depth(1)
            .resolve_auth(extension.account_id(), &authorization)
            .await
            .unwrap(),
        payload
    );
}

/// A NEP-141 transfer is shown with the required decimals and symbol and runs
/// only if the token's `ft_metadata()` has exactly these values.
async fn check_ft_metadata(
    near: &Near,
    wallet: &Wallet<WalletStellar>,
    recipient: &Wallet<WalletStellar>,
) {
    // near-sdk example token: decimals 24, symbol "EXAMPLE".
    let wasm = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../releases/fungible-token.wasm"),
    )
    .unwrap();
    let token: AccountId = format!("ft{}.{}", rand::random::<u32>(), near.account_id())
        .parse()
        .unwrap();
    near.transaction(&token)
        .create_account()
        .transfer(NearToken::from_near(5))
        .deploy(wasm.clone())
        .call("new_default_meta")
        .args(serde_json::json!({
            "owner_id": wallet.account_id(),
            "total_supply": NearToken::from_near(10).as_yoctonear().to_string(),
        }))
        .gas(Gas::from_tgas(30))
        .finish()
        .call("storage_deposit")
        .args(serde_json::json!({ "account_id": recipient.account_id() }))
        .deposit(NearToken::from_millinear(10))
        .gas(Gas::from_tgas(30))
        .send()
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();
    let transfer =
        |decimals, symbol| checked_ft_transfer(&token, recipient.account_id(), decimals, symbol);

    // Without the requirement, the amount cannot be shown and nothing is signed.
    let unchecked = NearPromise::new(token.clone()).function_call(
        FunctionCall::name("ft_transfer")
            .args(r#"{"receiver_id":"bob.near","amount":"1"}"#)
            .attach_deposit(NearToken::from_yoctonear(1)),
    );
    assert!(wallet.sign(unchecked).await.is_err());

    let (msg, _) = wallet.sign(transfer(24, "EXAMPLE")).await.unwrap();
    let message = request_signing_message(&wallet.public_key(), &msg).unwrap();
    assert!(message.contains(r#""amount": "1.5 EXAMPLE""#), "{message}");
    assert!(message.contains(r#""symbol": "EXAMPLE""#), "{message}");

    // A mismatch fails only the callback: the transfer is never created.
    for (decimals, symbol) in [(24, "EXAMPLE2"), (6, "EXAMPLE")] {
        let outcome = execute(near, wallet, transfer(decimals, symbol)).await;
        assert!(metadata_mismatch(&outcome), "{outcome:?}");
        assert_eq!(ft_balance(near, &token, recipient.account_id()).await, "0");
    }

    let outcome = execute(near, wallet, transfer(24, "EXAMPLE")).await;
    assert_eq!(
        ft_balance(near, &token, recipient.account_id()).await,
        NearToken::from_millinear(1500).as_yoctonear().to_string()
    );
    for receipt in &outcome.receipts_outcome {
        eprintln!(
            "ft_metadata check: {} burnt {}",
            receipt.outcome.executor_id, receipt.outcome.gas_burnt,
        );
    }

    check_ft_bootstrap(near, wallet, recipient, wasm).await;
}

/// A token that the checked promise itself creates with `StateInit` has no
/// metadata yet: nothing is created. It must be created in an earlier request.
async fn check_ft_bootstrap(
    near: &Near,
    wallet: &Wallet<WalletStellar>,
    recipient: &Wallet<WalletStellar>,
    wasm: Vec<u8>,
) {
    let ft_hash: [u8; 32] = Sha256::digest(&wasm).into();
    near.publish(wasm, PublishMode::Immutable)
        .await
        .unwrap()
        .result()
        .unwrap();
    let mut init = StateInitV1::code(ft_hash);
    // Unique account for each run, also on a reused sandbox.
    init.data
        .insert(b"test_run".to_vec(), rand::random::<[u8; 8]>().to_vec());
    let init = StateInit::from(init);
    let token = init.derive_account_id();
    let create = NearPromise::new(token.clone())
        .deterministic_state_init(init.clone(), NearToken::ZERO)
        // Storage for the token state written by `new_default_meta`.
        .transfer(NearToken::from_millinear(100))
        .function_call(
            FunctionCall::name("new_default_meta")
                .args(
                    serde_json::json!({
                        "owner_id": wallet.account_id(),
                        "total_supply": NearToken::from_near(10).as_yoctonear().to_string(),
                    })
                    .to_string(),
                )
                .gas(Gas::from_tgas(30)),
        )
        .function_call(
            FunctionCall::name("storage_deposit")
                .args(serde_json::json!({ "account_id": recipient.account_id() }).to_string())
                .attach_deposit(NearToken::from_millinear(10))
                .gas(Gas::from_tgas(30)),
        );
    let bootstrap = Request::new()
        .internal([WalletOp::require_ft_metadata(token.clone(), 24, "EXAMPLE")])
        .external([create
            .clone()
            .function_call(ft_transfer(recipient.account_id()))]);
    let outcome = execute(near, wallet, bootstrap).await;
    assert!(metadata_mismatch(&outcome), "{outcome:?}");
    assert!(near.account(&token).await.is_err());

    check_state_init_deposit(near, wallet, &init, recipient.account_id()).await;

    execute(near, wallet, create).await;
    let transfer = checked_ft_transfer(&token, recipient.account_id(), 24, "EXAMPLE");
    execute(near, wallet, transfer).await;
    assert_eq!(
        ft_balance(near, &token, recipient.account_id()).await,
        NearToken::from_millinear(1500).as_yoctonear().to_string()
    );
}

/// A `StateInit` deposit settles only current storage; excess is refunded even
/// when a later initializing call needs more storage in the same promise.
async fn check_state_init_deposit(
    near: &Near,
    wallet: &Wallet<WalletStellar>,
    init: &StateInit,
    refund_to: &AccountId,
) {
    let token = init.derive_account_id();
    let deposit = NearToken::from_near(1);
    let before = near.balance(refund_to).await.unwrap().total;
    let attempt = NearPromise::new(token.clone())
        .refund_to(refund_to.clone())
        .deterministic_state_init(init.clone(), deposit)
        .function_call(
            FunctionCall::name("new_default_meta")
                .args(
                    serde_json::json!({
                        "owner_id": wallet.account_id(),
                        "total_supply": "1000000",
                    })
                    .to_string(),
                )
                .gas(Gas::from_tgas(30)),
        );
    let outcome = execute(near, wallet, attempt).await;
    assert!(
        outcome
            .receipts_outcome
            .iter()
            .any(|r| { format!("{:?}", r.outcome.status).contains("LackBalanceForState") }),
        "{outcome:?}"
    );
    assert!(near.account(&token).await.is_err());
    assert_eq!(
        near.balance(refund_to).await.unwrap().total,
        before.saturating_add(deposit)
    );

    // The same deposit is entirely refunded on successful initialization of a
    // small ZBA. It never becomes a general-purpose balance at the receiver.
    execute(
        near,
        wallet,
        NearPromise::new(token.clone())
            .refund_to(refund_to.clone())
            .deterministic_state_init(init.clone(), deposit),
    )
    .await;
    assert_eq!(near.balance(&token).await.unwrap().total, NearToken::ZERO);
    assert_eq!(
        near.balance(refund_to).await.unwrap().total,
        before.saturating_add(deposit).saturating_add(deposit)
    );
}

/// `ft_transfer` of 1.5 tokens with 24 decimals to `receiver_id`.
fn ft_transfer(receiver_id: &AccountId) -> FunctionCall {
    FunctionCall::name("ft_transfer")
        .args(
            serde_json::json!({
                "receiver_id": receiver_id,
                "amount": NearToken::from_millinear(1500).as_yoctonear().to_string(),
            })
            .to_string(),
        )
        .attach_deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
}

/// [`ft_transfer`] that requires these token `decimals` and `symbol`.
fn checked_ft_transfer(
    token: &AccountId,
    receiver_id: &AccountId,
    decimals: u8,
    symbol: &str,
) -> Request {
    Request::new()
        .internal([WalletOp::require_ft_metadata(
            token.clone(),
            decimals,
            symbol,
        )])
        .external([NearPromise::new(token.clone()).function_call(ft_transfer(receiver_id))])
}

async fn ft_balance(near: &Near, token: &AccountId, account_id: &AccountId) -> String {
    near.view::<String>(token, "ft_balance_of")
        .args(serde_json::json!({ "account_id": account_id }))
        .await
        .unwrap()
}

fn metadata_mismatch(outcome: &FinalExecutionOutcome) -> bool {
    outcome.receipts_outcome.iter().any(|r| {
        format!("{:?}", r.outcome.status).contains("fungible token metadata does not match")
    })
}
