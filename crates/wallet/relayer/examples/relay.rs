use std::{env, fs, iter, path::Path, sync::LazyLock};

use defuse_wallet::Request;
use defuse_wallet_client::WalletClient;
use defuse_wallet_relayer::{RelayRequest, Relayer};
use ed25519_dalek::ed25519::signature::rand_core::OsRng;
use futures::{TryFutureExt, TryStreamExt, stream::FuturesUnordered};
use near_kit::{InMemorySigner, Near, SecretKey, sandbox::SandboxConfig};
use near_sdk::{GlobalContractId, NearToken, env::sha256_array};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_line_number(false)
                .with_file(false),
        )
        .with(EnvFilter::from_default_env())
        .init();

    let sandbox = SandboxConfig::builder().fresh().await;

    let near = sandbox.client();
    let global_contract_id = publish_global_contract(&near).await;
    let relayer = make_relayer(&near).await;

    let mut wallet = WalletClient::new(
        global_contract_id,
        ed25519_dalek::SigningKey::generate(&mut OsRng),
    )
    // .chain_id(relayer.client().chain_id().as_str())
    ;

    iter::repeat_with(|| {
        let (msg, proof) = wallet.sign(Request::new()).unwrap();
        relayer
            .relay(
                RelayRequest {
                    state_init: Some(wallet.state_init()),
                    msg,
                    proof,
                    min_gas: None,
                },
                NearToken::ZERO,
                None,
            )
            .map_ok(|_| ())
    })
    .take(10000)
    .collect::<FuturesUnordered<_>>()
    .try_collect::<()>()
    .await
    .unwrap();

    // println!("{r:#?}");
}

static WALLET_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let wasm = Path::new(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join("defuse-wallet.wasm");
    fs::read(wasm).expect("failed to read WASM")
});

async fn publish_global_contract(client: &Near) -> GlobalContractId {
    client
        .transaction(client.account_id())
        .publish_contract(WALLET_WASM.clone(), true)
        .await
        .unwrap();
    GlobalContractId::CodeHash(sha256_array(&*WALLET_WASM).into())
}

async fn make_relayer(client: &Near) -> Relayer {
    let relayer = Relayer::new(client.with_signer(generate_implicit_signer()));
    client
        .transfer(relayer.client().account_id(), NearToken::from_near(100))
        .await
        .unwrap();
    relayer
}

fn generate_implicit_signer() -> InMemorySigner {
    let secret_key = SecretKey::generate_ed25519();

    let account_id = hex::encode(secret_key.public_key().as_ed25519_bytes().unwrap());

    InMemorySigner::from_secret_key(account_id.parse().unwrap(), secret_key)
}
