use defuse_outlayer_grpc::{FILE_DESCRIPTOR_SET, GrpcConfig, OutlayerGrpc, OutlayerServiceServer};
use defuse_outlayer_service::{OutlayerConfig, Signer};
use defuse_outlayer_signer::InMemorySigner;

use anyhow::{Context as _, Result};
use config::{Config, Environment};
use serde::Deserialize;
use serde_with::hex::Hex;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tonic::transport::{Server, server::TcpIncoming};
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
    grpc_server: GrpcServerConfig,
    grpc: GrpcConfig,
    /// `tracing-subscriber` env-filter directives, e.g. `outlayer=info,defuse_outlayer_service=debug`.
    log: String,
}

#[serde_with::serde_as]
#[derive(Deserialize)]
#[serde(default)]
struct GrpcServerConfig {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    addr: SocketAddr,
    concurrency_limit_per_connection: usize,
}

impl Default for GrpcServerConfig {
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

    let config = AppConfig::load().context("config")?;

    {
        use tracing_subscriber::{
            EnvFilter,
            filter::{LevelFilter, filter_fn},
            fmt::{self, format::FmtSpan},
            prelude::*,
        };

        let logs = EnvFilter::new(&config.log);
        // Only collect span timings when the log level is DEBUG (or more verbose).
        let timings = logs
            .max_level_hint()
            .is_some_and(|level| level >= LevelFilter::DEBUG)
            .then(|| {
                // Busy/idle timing for the executor's `compile`/`execute` spans, the
                // gRPC adapter's `grpc` total span, and the service's per-request spans
                // (`resolve_url`, `fetch`). `is_span` keeps it to timing lines; the
                // target filter scopes it to those spans rather than the whole tree.
                fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_filter(filter_fn(|meta| {
                        meta.is_span()
                            && (meta.target().starts_with("defuse_outlayer_executor")
                                || meta.target().starts_with("defuse_outlayer_grpc")
                                || meta.target().starts_with("defuse_outlayer_service"))
                    }))
            });

        tracing_subscriber::registry()
            .with(fmt::layer().with_filter(logs))
            .with(timings)
            .init();
    }

    // TODO: derive seed from CKD
    #[allow(clippy::option_if_let_else)]
    let signer = match config.seed {
        Some(seed) => {
            tracing::warn!("using custom seed — not intended for production use");
            InMemorySigner::from_seed(&seed)
        }
        None => unimplemented!("seed must be provided until CKD integration is complete"),
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

    let incoming = TcpIncoming::bind(config.grpc_server.addr).context("bind")?;
    tracing::info!(addr = %incoming.local_addr().context("local addr")?, "gRPC listening");

    Server::builder()
        .concurrency_limit_per_connection(config.grpc_server.concurrency_limit_per_connection)
        .add_service(health_service)
        .add_service(grpc_service)
        .add_service(reflection_service)
        .serve_with_incoming(incoming)
        .await
        .context("server error")
}
