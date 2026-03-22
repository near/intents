use std::{env, fs, iter, path::Path, sync::LazyLock};

use defuse_wallet::Request;
use defuse_wallet_client::WalletClient;
use defuse_wallet_relayer::{RelayRequest, Relayer};
use ed25519_dalek::ed25519::signature::rand_core::OsRng;
use futures::{StreamExt, TryFutureExt, TryStreamExt, stream};
use near_kit::{PublishMode, sandbox::SandboxConfig};
use near_sdk::{GlobalContractId, NearToken, env::sha256_array};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

static WALLET_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let wasm = Path::new(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join("defuse-wallet.wasm");
    fs::read(wasm).expect("failed to read WASM")
});

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
    near.publish(WALLET_WASM.clone(), PublishMode::Immutable)
        .await
        .unwrap();
    let global_contract_id = GlobalContractId::CodeHash(sha256_array(&*WALLET_WASM).into());
    let relayer = Relayer::new(near);

    let mut wallet = WalletClient::new(
        global_contract_id,
        ed25519_dalek::SigningKey::generate(&mut OsRng),
    )
    // .chain_id(relayer.client().chain_id().as_str())
    ;

    stream::iter(
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
        .take(10_000),
    )
    .buffer_unordered(1000)
    .try_collect::<()>()
    .await
    .unwrap();

    // println!("{r:#?}");
}
