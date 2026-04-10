#![allow(clippy::cast_precision_loss, clippy::as_conversions)]

use std::{env, fs, iter, path::Path, sync::LazyLock};

use defuse_wallet::Request;
use defuse_wallet_relayer::{RelayRequest, Relayer};
use defuse_wallet_sdk::WalletSigner;
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

#[tokio::main]
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

    let global_contract_id = {
        near.publish(WALLET_WASM.clone(), PublishMode::Immutable)
            .wait_until(near_kit::TxExecutionStatus::Final)
            .await
            .unwrap();
        GlobalContractId::CodeHash(sha256_array(&*WALLET_WASM).into())
    };

    let relayer = Relayer::new(near.clone());

    let mut wallet = WalletSigner::new(
        global_contract_id,
        ed25519_dalek::SigningKey::generate(&mut OsRng),
    )
    // TODO
    // .chain_id(relayer.client().chain_id().as_str())
    ;

    let txs_count = 10_000;

    let txs = stream::iter(
        iter::repeat_with(|| {
            let (msg, proof) = wallet.sign(Request::new()).unwrap();
            relayer
                .w_execute_signed(
                    RelayRequest {
                        state_init: Some(wallet.state_init()),
                        msg,
                        proof,
                        min_gas: None,
                    },
                    NearToken::ZERO,
                    None,
                )
                .inspect_ok(|r| tracing::info!(tx.hash = %r.transaction_hash(), tx.gas_used = %r.total_gas_used()))
                .map_ok(|_| ())
        })
        .take(txs_count),
    )
    .buffer_unordered(1000);

    let started_at = tokio::time::Instant::now();
    txs.try_collect::<()>().await.unwrap();

    println!(
        "avg: {} TPS",
        txs_count as f32 / started_at.elapsed().as_secs_f32()
    );
}
