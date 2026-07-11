#![allow(clippy::cast_precision_loss, clippy::as_conversions)]

use std::{env, fs, path::Path, sync::LazyLock};

use defuse_digest::{Digest, sha2::Sha256};
use defuse_wallet_ed25519::WalletEd25519;
use defuse_wallet_relayer::{WalletRelayRequest, WalletRelayer};
use defuse_wallet_sdk::{MAINNET, NearToken, Request, Wallet};
use ed25519_dalek::ed25519::signature::rand_core::OsRng;
use futures::{StreamExt, TryStreamExt, stream};
use near_kit::{Final, GlobalContractId, PublishMode, sandbox::SandboxConfig};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

static WALLET_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let wasm = Path::new(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join("defuse-wallet-ed25519.wasm");
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
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .unwrap();
        GlobalContractId::CodeHash(Sha256::digest(&*WALLET_WASM).into())
    };

    let relayer = WalletRelayer::new(near.clone());

    let wallet = Wallet::<WalletEd25519, _>::new(
        global_contract_id,
        ed25519_dalek::SigningKey::generate(&mut OsRng),
    );

    let started_at = tokio::time::Instant::now();
    let txs_count = 10_000;

    stream::iter(0..txs_count)
        // TODO: relayer.client().chain_id().as_str()
        .then(|_n| wallet.sign(Request::new(), MAINNET))
        .err_into()
        .map_ok(|(msg, proof)| {
            relayer.w_execute_signed(
                WalletRelayRequest {
                    deterministic_state_init: Some(wallet.deterministic_state_init().clone()),
                    msg,
                    proof,
                    gas: None,
                },
                NearToken::ZERO,
                None,
            )
        })
        .try_buffer_unordered(500)
        .map_ok(|r| {
            assert!(r.is_success());

            tracing::info!(
                tx.hash = %r.transaction_hash(),
                tx.gas_used = %r.total_gas_used()
            );
        })
        .try_collect::<()>()
        .await
        .unwrap();

    println!(
        "avg: {} TPS",
        txs_count as f32 / started_at.elapsed().as_secs_f32()
    );
}
