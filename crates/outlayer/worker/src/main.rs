use defuse_outlayer_host::crypto::Signer;
use defuse_outlayer_proto::{FILE_DESCRIPTOR_SET, outlayer_service_server::OutlayerServiceServer};
use defuse_outlayer_service::{OutlayerConfig, OutlayerGrpc};
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
use tower::ServiceBuilder;
use zeroize::Zeroizing;

const PREFIX: &str = "WORKER";
const DEFAULT_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 50051));
const DEFAULT_MAX_PARALLEL_WASM_EXECUTIONS: usize = 2;
const DEFAULT_CONNECTIONS_LIMIT: usize = 500;
const DEFAULT_CONCURRENCY_LIMIT_PER_CONNECTION: usize = 1;
const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

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
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    execution_timeout_s: Duration,
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
            execution_timeout_s: DEFAULT_EXECUTION_TIMEOUT,
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
        ServiceBuilder::new()
            // When the queue is full, immediately return RESOURCE_EXHAUSTED so the load
            // balancer can route the request elsewhere rather than waiting here.
            // RESOURCE_EXHAUSTED is the standard gRPC backpressure code respected by
            // gRPC-aware load balancers (e.g. Envoy, AWS ALB).
            .load_shed()
            // Bounded async queue. Decouples acceptance from execution. When full, the
            // load_shed layer above fires.
            .buffer(config.connections_limit)
            // Limit concurrent WASM executions. WASM runs synchronously on blocking
            // threads; this prevents saturating the thread pool with CPU-bound work.
            .concurrency_limit(config.max_parallel_wasm_executions)
            // Deadline for a single execution. Runs inside the buffer's background
            // worker, so it actually cancels async work (e.g. slow WASM downloads)
            // on expiry.
            // TODO: spawn_blocking phases (compile, WASM run) cannot be
            // interrupted consider using epoch interruptions on wasm execution
            .timeout(config.execution_timeout_s)
            .service(outlayer),
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
