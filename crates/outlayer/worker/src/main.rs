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
    time::Duration,
};
use tonic::transport::Server;
use tonic_health::ServingStatus;
use zeroize::Zeroizing;

const PREFIX: &str = "WORKER";
const DEFAULT_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 50051));
const DEFAULT_MAX_PARALLEL_WASM_EXECUTIONS: usize = 2;
const DEFAULT_CONNECTIONS_LIMIT: usize = 500;
const DEFAULT_CONCURRENCY_LIMIT_PER_CONNECTION: usize = 1;
const DEFAULT_REQUEST_HANDLING_TIMEOUT: Duration = Duration::from_secs(30);

#[serde_with::serde_as]
#[derive(Deserialize)]
#[serde(default)]
struct AppConfig {
    #[serde(rename = "service")]
    outlayer: OutlayerConfig,
    #[serde_as(as = "Option<Hex>")]
    seed: Option<Zeroizing<Vec<u8>>>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    addr: SocketAddr,
    max_parallel_wasm_executions: usize,
    connections_limit: usize,
    concurrency_limit_per_connection: usize,
    #[serde(rename = "request_handling_timeout_seconds")]
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    request_handling_timeout: Duration,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            outlayer: OutlayerConfig::default(),
            seed: None,
            addr: DEFAULT_ADDR,
            max_parallel_wasm_executions: DEFAULT_MAX_PARALLEL_WASM_EXECUTIONS,
            connections_limit: DEFAULT_CONNECTIONS_LIMIT,
            concurrency_limit_per_connection: DEFAULT_CONCURRENCY_LIMIT_PER_CONNECTION,
            request_handling_timeout: DEFAULT_REQUEST_HANDLING_TIMEOUT,
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
            .context("config")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config = AppConfig::load()?;

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

    let outlayer = config.outlayer.build(signer).context("outlayer")?;

    let grpc_service = OutlayerServiceServer::new(OutlayerGrpc::new(
        outlayer,
        GrpcConfig {
            connections_limit: config.connections_limit,
            max_parallel_wasm_executions: config.max_parallel_wasm_executions,
            request_handling_timeout: config.request_handling_timeout,
        },
    ));

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

    tracing::info!(addr = %config.addr, "listening");

    Server::builder()
        .concurrency_limit_per_connection(config.concurrency_limit_per_connection)
        .add_service(health_service)
        .add_service(grpc_service)
        .add_service(reflection_service)
        .serve(config.addr)
        .await
        .context("server error")
}
