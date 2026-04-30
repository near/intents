pub mod config;
pub mod env;
pub mod error;
pub mod executor;
pub mod on_chain;
pub mod resolver;
pub mod service;
pub mod signing_service;
pub mod storage;
pub mod types;
pub mod utils;

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use defuse_outlayer_crypto::signer::InMemorySigner;
use lru::LruCache;
use tower::{Service, ServiceBuilder, service_fn};

use defuse_outlayer_host::HostFunctions;
use defuse_outlayer_vm_runner::VmRuntime;

pub use config::{Config, FetchConfig};
pub use error::ExecutionStackError;
pub use executor::{WasmExecutor, WasmExecutorConfig};
pub use on_chain::{OnChainFetchError, OnChainFetchService};
pub use resolver::HttpResolver;
pub use service::OutlayerService;
pub use signing_service::{SignedExecutionResponse, SigningService};
pub use types::{ExecutionRequest, ExecutionResponse, OffChainRequest, OnChainRequest, Request};
pub use utils::cache::CacheConfig;

use utils::cache::CacheLayer;
use utils::retry::Attempts;
use utils::timeout_err;

pub fn build_stack<H, Req>(
    signing_key: InMemorySigner,
    runtime: Arc<VmRuntime<H>>,
    config: Config,
    host_template: H,
) -> impl Service<Req, Response = SignedExecutionResponse, Error = ExecutionStackError> + Send + 'static
where
    H: HostFunctions + Clone + Send + Sync + 'static,
    Req: Into<Request> + Clone + Send + 'static,
{
    let executor = WasmExecutor::new(Arc::clone(&runtime), config.executor, host_template);

    let wasm_cache = Arc::new(Mutex::new(LruCache::<[u8; 32], _>::new(
        config.cache.capacity,
    )));

    let wasm = ServiceBuilder::new()
        .layer(CacheLayer::new(wasm_cache))
        .and_then(move |bytes: Arc<Bytes>| {
            let rt = Arc::clone(&runtime);
            async move {
                tokio::task::spawn_blocking(move || {
                    rt.compile(&*bytes).map_err(ExecutionStackError::Compile)
                })
                .await
                .map_err(|e| ExecutionStackError::Compile(anyhow::anyhow!(e)))?
            }
        })
        .map_err(timeout_err::<resolver::ResolveError>)
        .retry(Attempts(config.fetch.retry_attempts))
        .timeout(config.fetch.timeout)
        .service(resolver::ResolverService::new(resolver::build_resolver(
            config.cache.max_fetch_bytes,
        )));

    let env = ServiceBuilder::new()
        .map_err(timeout_err::<env::EnvFetchError>)
        .retry(Attempts(config.fetch.retry_attempts))
        .timeout(config.fetch.timeout)
        .service(env::EnvFetchService);

    let storage = ServiceBuilder::new()
        .map_err(timeout_err::<storage::StorageFetchError>)
        .retry(Attempts(config.fetch.retry_attempts))
        .timeout(config.fetch.timeout)
        .service(storage::StorageFetchService);

    // TODO: consider moving signing outside of the Tower stack
    ServiceBuilder::new()
        .map_err(timeout_err::<ExecutionStackError>)
        .timeout(config.total_timeout)
        .service(SigningService::new(
            OutlayerService::new(
                executor,
                wasm,
                env,
                storage,
                service_fn(|_: OffChainRequest| async {
                    Err::<ExecutionRequest, _>(ExecutionStackError::Internal(
                        "off-chain requests not supported in this stack".into(),
                    ))
                }),
            ),
            signing_key,
        ))
}
