mod cli;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::{Deserialize, Serialize};
use serde_with::hex::Hex;

use defuse_outlayer_executor::InMemorySigner;
use defuse_outlayer_service::{Outlayer, OutlayerConfig};
use tokio::sync::mpsc;
use tower::{Service as _, ServiceExt as _};

#[serde_with::serde_as]
#[derive(Deserialize, Serialize)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    signer_seed: Option<Vec<u8>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            outlayer: OutlayerConfig::default(),
            signer_seed: None,
        }
    }
}

type Request = (defuse_outlayer_service::Code<'static>, bytes::Bytes);

#[tokio::main]
async fn main() -> Result<()> {
    //TODO: remove
    if std::env::args().any(|a| a == "--print-config") {
        cli::print_env_vars();
        return Ok(());
    }

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config: AppConfig = Config::builder()
        .add_source(
            Environment::with_prefix("WORKER")
                .prefix_separator("__")
                .separator("__"),
        )
        .build()
        .and_then(|c| c.try_deserialize())
        .context("config")?;

    let signer = match config.signer_seed {
        Some(ref seed) => {
            tracing::warn!("using custom signer seed — not intended for production use");
            InMemorySigner::from_seed(seed)
        }
        // TODO: derive seed from CKD
        None => unimplemented!("signer seed must be provided until CKD integration is complete"),
    };

    let mut svc = Outlayer::builder()
        .with_config(config.outlayer)
        .build_service(signer)
        .context("outlayer")?;

    let (_requests_tx, mut requests_rx) = mpsc::channel::<Request>(100);
    let (result_tx, mut _result_rx) = mpsc::channel(100);
    loop {
        svc.ready().await.map_err(|e| anyhow::anyhow!(e))?;
        let req = requests_rx.recv().await.expect("should be infinite");
        let fut = svc.call(req); // future is 'static, svc stays here
        let result_tx = result_tx.clone();
        tokio::spawn(async move {
            let resp = fut.await;
            result_tx.send(resp).await.ok();
        });
    }
}
