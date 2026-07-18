use defuse_wallet_ed25519::{WalletEd25519, WalletEd25519Signer, crypto::ed25519::ed25519_dalek};
use defuse_wallet_relayer::{WalletRelayRequest, WalletRelayer};
use defuse_wallet_sdk::{
    NearPromise, Request, Wallet, WalletOp, WalletSigner, actions::FunctionCall,
};
use near_kit::{AccountIdRef, Gas, Near, NearToken};
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use serde_json::json;

const WALLET_GLOBAL_CONTRACT_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0sb0d7ef4f935c6ef78e08ad03569767aaec4223a3");
const MPC_ACCOUNT_ID: &AccountIdRef = AccountIdRef::new_or_panic("v1.signer");

const EXAMPLE_EXTENSION: &AccountIdRef = AccountIdRef::new_or_panic("extension.near");

#[tokio::main]
async fn main() {
    // 0.0) Generate a keypair
    let signer = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
    // 0.1) Build wallet state with signer's public key
    let wallet = Wallet::<WalletEd25519>::new(
        WALLET_GLOBAL_CONTRACT_ID.to_owned(),
        WalletEd25519Signer(signer),
    );
    println!("public key: {}", wallet.public_key());
    // 0.2) Derive wallet account_id
    println!("wallet.account_id() = {}", wallet.account_id());

    // 1) Prepare wallet request
    let wallet_request = Request::new().internal([
        // add extension as just a showcase
        WalletOp::AddExtension { account_id: EXAMPLE_EXTENSION.to_owned() },
        // remove it immediately after
        WalletOp::RemoveExtension { account_id: EXAMPLE_EXTENSION.to_owned() },
    ]).external([NearPromise::new(MPC_ACCOUNT_ID).function_call(
        FunctionCall::name("sign")
            .args_json(json!({
                "request": {
                    "payload_v2": {
                        "Ecdsa": "0128fdba02691843069aba70c0523b9c43f4b0de4e34962462b0525490780a53"
                    },
                    "domain_id": 0,
                    "path": ""
                }
            }))
            .attach_deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(30)),
    )]);
    println!(
        "wallet_request: {}",
        serde_json::to_string_pretty(&wallet_request).unwrap()
    );

    // 2) Sign wallet request
    let (msg, proof) = wallet.sign(wallet_request).await.unwrap();

    // 3) Build
    let relayer_request = WalletRelayRequest::new(msg, proof)
        // 3.a) (optional) initialize the wallet on first tx
        .deterministic_state_init(wallet.deterministic_state_init().clone());
    println!(
        "relayer_request: {}",
        serde_json::to_string_pretty(&relayer_request).unwrap()
    );

    let relayer = WalletRelayer::new(Near::from_env().unwrap());
    println!("relayer_id: {}", relayer.client().account_id());

    // 4) Send request to relayer
    let tx = relayer
        .w_execute_signed(relayer_request, NearToken::ZERO, None)
        .await
        .unwrap();

    // 5) Get transaction hash and MPC signature (parsed by relayer)
    println!("tx hash: {}", tx.transaction_hash());
}
