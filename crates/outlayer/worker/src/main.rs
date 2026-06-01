use std::sync::Arc;

use defuse_outlayer_service::{OutlayerConfig, host::crypto::Signer};
use defuse_outlayer_signer::InMemorySigner;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::Deserialize;
use serde_with::hex::Hex;
use zeroize::Zeroizing;

const PREFIX: &str = "WORKER";

#[serde_with::serde_as]
#[derive(Deserialize, Default)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    seed: Option<Zeroizing<Vec<u8>>>,
}

impl AppConfig {
    fn load() -> Result<Self> {
        Config::builder()
            .add_source(
                Environment::with_prefix(PREFIX)
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()
            .and_then(config::Config::try_deserialize)
            .context("config")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("OUTLAYER_LOG"))
        .init();

    let AppConfig { outlayer, seed } = AppConfig::load()?;

    // TODO: derive seed from CKD
    #[allow(clippy::option_if_let_else)]
    let signer = match seed.as_deref() {
        Some(seed) => {
            tracing::warn!("using custom signer seed — not intended for production use");
            InMemorySigner::from_seed(seed)
        }
        None => unimplemented!("signer seed must be provided until CKD integration is complete"),
    };

    let signer: Arc<dyn Signer> = Arc::new(signer);
    let _ = outlayer.build(signer).context("outlayer")?;

    Ok(())
}
