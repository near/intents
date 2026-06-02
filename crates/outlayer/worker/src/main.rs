use defuse_outlayer_grpc::{FILE_DESCRIPTOR_SET, GrpcConfig, OutlayerGrpc, OutlayerServiceServer};
use defuse_outlayer_host::crypto::Signer;
use defuse_outlayer_service::OutlayerConfig;
use defuse_outlayer_signer::InMemorySigner;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::Deserialize;
use serde_with::hex::Hex;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tonic::transport::Server;
use tonic_health::ServingStatus;
use zeroize::Zeroizing;

const PREFIX: &str = "OUTLAYER";
const DEFAULT_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 50051));
const DEFAULT_CONCURRENCY_LIMIT_PER_CONNECTION: usize = 1;

#[serde_with::serde_as]
#[derive(Deserialize, Default)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    seed: Option<Zeroizing<Vec<u8>>>,
    http_server: HttpServerConfig,
    grpc: GrpcConfig,
}

#[serde_with::serde_as]
#[derive(Deserialize)]
#[serde(default)]
struct HttpServerConfig {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    addr: SocketAddr,
    concurrency_limit_per_connection: usize,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            addr: DEFAULT_ADDR,
            concurrency_limit_per_connection: DEFAULT_CONCURRENCY_LIMIT_PER_CONNECTION,
        }
    }
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
            .map_err(Into::into)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config = AppConfig::load().context("config")?;

    // TODO: derive seed from CKD
    #[allow(clippy::option_if_let_else)]
    let signer = match config.seed {
        Some(seed) => {
            tracing::warn!("using custom signer seed — not intended for production use");
            InMemorySigner::from_seed(&seed)
        }
        None => unimplemented!("signer seed must be provided until CKD integration is complete"),
    };

    let signer: Arc<dyn Signer> = Arc::new(signer);

    let outlayer = config.outlayer.build(signer).context("build")?;

    let grpc_service = OutlayerServiceServer::new(OutlayerGrpc::new(outlayer, config.grpc));

    // Implements the standard gRPC health checking protocol (grpc.health.v1).
    // Kubernetes uses this natively for liveness/readiness probes.
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status("", ServingStatus::Serving)
        .await;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    tracing::info!(addr = %config.http_server.addr, "listening");

    Server::builder()
        .concurrency_limit_per_connection(config.http_server.concurrency_limit_per_connection)
        .add_service(health_service)
        .add_service(grpc_service)
        .add_service(reflection_service)
        .serve(config.http_server.addr)
        .await
        .context("server error")
}
