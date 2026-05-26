use std::sync::Arc;

use defuse_outlayer_host::crypto::Signer;
use defuse_outlayer_service::{Outlayer, OutlayerConfig};
use defuse_outlayer_signer::InMemorySigner;
use tonic::transport::Server;
use tonic_health::ServingStatus;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::hex::Hex;

const PREFIX: &str = "WORKER";

#[serde_with::serde_as]
#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    signer_seed: Option<Vec<u8>>,
    #[serde(default = "default_addr")]
    addr: String,
}

fn default_addr() -> String {
    "0.0.0.0:50051".to_owned()
}

//TODO: probably to be removed, but lets keep it until configs
//are more stable
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

#[tokio::main]
async fn main() -> Result<()> {
    //TODO: is that even needed?
    if std::env::args().any(|a| a == "--print-env") {
        let defaults = serde_json::to_value(AppConfig::default()).unwrap();
        print_value(PREFIX, &defaults);
        return Ok(());
    }

    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config: AppConfig = Config::builder()
        .add_source(
            Environment::with_prefix(PREFIX)
                .prefix_separator("__")
                .separator("__"),
        )
        .build()
        .and_then(config::Config::try_deserialize)
        .context("config")?;

    // TODO: derive seed from CKD
    #[allow(clippy::option_if_let_else)]
    let signer = match config.signer_seed {
        Some(ref seed) => {
            tracing::warn!("using custom signer seed — not intended for production use");
            InMemorySigner::from_seed(seed)
        }
        None => unimplemented!("signer seed must be provided until CKD integration is complete"),
    };

    let signer: Arc<dyn Signer> = Arc::new(signer);

    let grpc_service = Outlayer::builder()
        .with_config(config.outlayer)
        .build_service(signer)
        .context("outlayer")?;

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status("", ServingStatus::Serving)
        .await;

    let addr = config.addr.parse().context("invalid listen address")?;
    tracing::info!(%addr, "listening");

    Server::builder()
        .add_service(health_service)
        .add_service(grpc_service)
        .serve(addr)
        .await
        .context("server error")
}
