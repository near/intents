use defuse_wallet_ed25519::WalletEd25519;
use defuse_wallet_relayer::{WalletRelayRequest, WalletRelayer};
use defuse_wallet_sdk::{MAINNET, NearPromise, Request, Wallet, WalletOp, actions::FunctionCall};
use ed25519_dalek::ed25519::signature::rand_core::OsRng;
use near_kit::{AccountIdRef, Gas, Near, NearToken};
use serde_json::json;

const WALLET_GLOBAL_CONTRACT_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0sb0d7ef4f935c6ef78e08ad03569767aaec4223a3");
const MPC_ACCOUNT_ID: &AccountIdRef = AccountIdRef::new_or_panic("v1.signer");

const EXAMPLE_EXTENSION: &AccountIdRef = AccountIdRef::new_or_panic("extension.near");

#[tokio::main]
async fn main() {
    // 0.0) Generate keypair
    let wallet_signer = ed25519_dalek::SigningKey::generate(&mut OsRng);
    println!(
        "wallet_keypair: 'ed25519:{}'",
        bs58::encode(wallet_signer.to_keypair_bytes()).into_string()
    );

    // 0.1) Build wallet state
    let wallet =
        Wallet::<WalletEd25519, _>::new(WALLET_GLOBAL_CONTRACT_ID.to_owned(), wallet_signer);

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
    let (msg, proof) = wallet.sign(wallet_request, MAINNET).await.unwrap();

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
