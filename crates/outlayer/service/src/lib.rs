pub mod cache;
pub mod env_fetch;
pub mod error;
pub mod executor;
pub mod resolver;
pub mod storage_fetch;
pub mod tee_worker;
pub mod types;

use std::future::Ready;
use std::num::NonZeroUsize;
use std::time::Duration;

use tower::retry::Policy;
use tower::{BoxError, Service, ServiceBuilder};

pub use error::ExecutionStackError;
pub use executor::WasmExecutor;
pub use resolver::HttpResolver;
pub use tee_worker::TeeWorkerService;
pub use types::{ExecutionResponse, Request, SignedExecutionResponse, WorkerSigningKey};

pub const DEFAULT_CACHE_CAPACITY: usize = 100;
pub const DEFAULT_MAX_FETCH_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

#[derive(Clone)]
struct Attempts(usize);

impl<Req: Clone, Res, E> Policy<Req, Res, E> for Attempts {
    type Future = Ready<()>;

    fn retry(&mut self, _: &mut Req, result: &mut Result<Res, E>) -> Option<Self::Future> {
        result.is_err().then_some(())?;
        self.0 = self.0.checked_sub(1)?;
        Some(std::future::ready(()))
    }

    fn clone_request(&mut self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

// tower::TimeoutLayer always boxes both the inner service error and tower::timeout::error::Elapsed
// into BoxError, forcing a runtime downcast at the consumer side. The bound `E: Into<ExecutionStackError>`
// provides a compile-time check that the expected error type is wired into the stack correctly.
//
// TODO: replace tower::TimeoutLayer with a typed wrapper that preserves S::Error, or migrate the
// error types to anyhow to avoid the downcast pattern entirely.
fn timeout_err<E>(e: BoxError) -> ExecutionStackError
where
    E: std::error::Error + Send + Sync + 'static + Into<ExecutionStackError>,
{
    if e.is::<tower::timeout::error::Elapsed>() {
        ExecutionStackError::Timeout
    } else {
        (*e.downcast::<E>().unwrap()).into()
    }
}

/// Builds the composed Tower stack.
///
/// Stack: `map_err` → `timeout(total)` → `map_response(sign)` → `TeeWorkerService`
///
/// `TeeWorkerService` fans out in parallel:
///   - resolver: `CacheLayer` → `map_err` → `RetryLayer` → `timeout(per-attempt)` → `Steer[Http, Inline]`
///   - env:     `map_err` → `RetryLayer` → `TimeoutLayer` → `EnvFetchService`
///   - storage: `map_err` → `RetryLayer` → `TimeoutLayer` → `StorageFetchService`
pub fn build_stack(
    signing_key: WorkerSigningKey,
    total_timeout: Duration,
    fetch_timeout: Duration,
    env_storage_timeout: Duration,
    retry_attempts: usize,
) -> impl Service<Request, Response = SignedExecutionResponse, Error = ExecutionStackError>
       + Send
       + 'static {
    let resolver = ServiceBuilder::new()
        .layer(cache::CacheLayer::new(
            NonZeroUsize::new(DEFAULT_CACHE_CAPACITY).unwrap(),
        ))
        .map_err(timeout_err::<resolver::ResolveError>)
        .retry(Attempts(retry_attempts))
        .timeout(fetch_timeout)
        .service(resolver::build_resolver(DEFAULT_MAX_FETCH_BYTES));

    let env = ServiceBuilder::new()
        .map_err(timeout_err::<env_fetch::EnvFetchError>)
        .retry(Attempts(retry_attempts))
        .timeout(env_storage_timeout)
        .service(env_fetch::EnvFetchService);

    let storage = ServiceBuilder::new()
        .map_err(timeout_err::<storage_fetch::StorageFetchError>)
        .retry(Attempts(retry_attempts))
        .timeout(env_storage_timeout)
        .service(storage_fetch::StorageFetchService);

    ServiceBuilder::new()
        .map_err(timeout_err::<ExecutionStackError>)
        .timeout(total_timeout)
        .map_response(move |r| signing_key.sign(r))
        .service(TeeWorkerService::new(WasmExecutor::new(), resolver, env, storage))
}
