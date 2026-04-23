pub mod config;
pub mod env;
pub mod error;
pub mod executor;
pub mod resolver;
pub mod service;
pub mod sign;
pub mod storage;
pub mod types;
pub mod utils;

use std::sync::Arc;

use bytes::Bytes;
use tower::{Service, ServiceBuilder};

use defuse_outlayer_host_functions::HostFunctions;
use defuse_outlayer_vm_runner::VmRuntime;

pub use config::{Config, FetchConfig};
pub use error::ExecutionStackError;
pub use executor::{WasmExecutor, WasmExecutorConfig};
pub use resolver::HttpResolver;
pub use service::OutlayerService;
pub use sign::{SignedExecutionResponse, WorkerSigningKey};
pub use types::{ExecutionResponse, Request};
pub use utils::cache::CacheConfig;

use utils::cache::CacheLayer;
use utils::retry::Attempts;
use utils::timeout_err;
pub fn build_stack<H>(
    signing_key: WorkerSigningKey,
    runtime: Arc<VmRuntime<H>>,
    config: Config,
) -> impl Service<Request, Response = SignedExecutionResponse, Error = ExecutionStackError>
+ Send
+ 'static
where
    H: HostFunctions + Default + Send + Sync + 'static,
{
    let executor = WasmExecutor::new(Arc::clone(&runtime), config.executor);

    let wasm = ServiceBuilder::new()
        .layer(CacheLayer::new(config.cache.capacity))
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

    ServiceBuilder::new()
        .map_err(timeout_err::<ExecutionStackError>)
        .timeout(config.total_timeout)
        .map_response(move |r| signing_key.sign(r))
        .service(OutlayerService::new(executor, wasm, env, storage))
}
