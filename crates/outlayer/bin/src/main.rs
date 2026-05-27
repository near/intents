use std::sync::Arc;

use defuse_outlayer_host::crypto::Signer;
use defuse_outlayer_service::{OutlayerBuilder, OutlayerConfig};
use defuse_outlayer_signer::InMemorySigner;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::hex::Hex;
use zeroize::Zeroizing;

const PREFIX: &str = "WORKER";

#[serde_with::serde_as]
#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    signer_seed: Option<Zeroizing<Vec<u8>>>,
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

    //TODO: probably to be removed, but lets keep it until configs
    //are more stable
    fn print_defaults() {
        fn print_value(prefix: &str, value: &Value) {
            match value {
                Value::Object(map) => {
                    for (key, val) in map {
                        print_value(&format!("{prefix}__{}", key.to_uppercase()), val);
                    }
                }
                Value::Null => println!("# {prefix}="),
                other => println!("{prefix}={other}"),
            }
        }
        let defaults = serde_json::to_value(Self::default()).unwrap();
        print_value(PREFIX, &defaults);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--print-env") {
        AppConfig::print_defaults();
        return Ok(());
    }

    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config = AppConfig::load()?;

    let AppConfig {
        outlayer,
        signer_seed,
    } = config;

    // TODO: derive seed from CKD
    #[allow(clippy::option_if_let_else)]
    let signer = match signer_seed.as_deref() {
        Some(seed) => {
            tracing::warn!("using custom signer seed — not intended for production use");
            InMemorySigner::from_seed(seed)
        }
        None => unimplemented!("signer seed must be provided until CKD integration is complete"),
    };

    let signer: Arc<dyn Signer> = Arc::new(signer);
    let _ = OutlayerBuilder::default()
        .with_config(outlayer)
        .build(signer)
        .context("outlayer")?;

    Ok(())
}
