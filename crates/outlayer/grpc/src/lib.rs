mod convert;

use std::time::Duration;

use bytes::Bytes;
use defuse_outlayer_executor::Outcome;
use defuse_outlayer_proto as proto;
use defuse_outlayer_proto::outlayer_service_server::OutlayerService;
use defuse_outlayer_service::{Code, Outlayer};
use tonic::{Request, Response, Status};
use tower::util::BoxCloneSyncService;
use tower::{BoxError, ServiceBuilder, ServiceExt as _, service_fn};

use crate::convert::ProtoTryFrom as _;

pub use defuse_outlayer_proto::FILE_DESCRIPTOR_SET;
pub use defuse_outlayer_proto::outlayer_service_server::OutlayerServiceServer;

/// Request handled by the layered execution service.
pub struct ExecuteRequest {
    pub app: Code<'static>,
    pub input: Bytes,
    pub fuel: Option<u64>,
}

/// The fully-layered execution service, type-erased so it can be named.
//
// Type erasure is forced by the `buffer` layer: `Buffer<Req, F>` is generic over
// the inner service's (unnameable) future type, so the stack can't be spelled out;
// `BoxCloneSyncService` also satisfies tonic's `Clone + Send + Sync` requirement.
// TODO: replace `buffer` with a dedicated layer that caps N active executions via a
// semaphore acquire (instead of queueing requests). Such a stack would be nameable
// and could drop this erasure — out of scope for this PR.
type LayeredService = BoxCloneSyncService<ExecuteRequest, Outcome, BoxError>;

#[derive(Clone)]
pub struct OutlayerGrpc(LayeredService);

impl OutlayerGrpc {
    /// Builds the gRPC adapter, wrapping `outlayer` with the tower layer stack
    /// (`load_shed` → `buffer` → `concurrency_limit` → `timeout`).
    pub fn new(outlayer: Outlayer, config: GrpcConfig) -> Self {
        let service = ServiceBuilder::new()
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
            .timeout(config.request_handling_timeout)
            // Thin adapter: turn `Outlayer` into a `tower::Service` over `ExecuteRequest`.
            // Kept here (the transport edge) so the core `service` crate stays free of tower.
            .service(service_fn(move |req: ExecuteRequest| {
                let outlayer = outlayer.clone();
                async move { outlayer.execute(req.app, req.input, req.fuel).await }
            }));

        Self(BoxCloneSyncService::new(service))
    }
}

#[tonic::async_trait]
impl OutlayerService for OutlayerGrpc {
    #[tracing::instrument(name = "grpc", skip_all)]
    async fn call(
        &self,
        request: Request<proto::Request>,
    ) -> Result<Response<proto::Response>, Status> {
        let svc_req = ExecuteRequest::proto_try_from(request.into_inner())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // The `buffer` layer (and `load_shed`/`timeout`) erase errors into `BoxError`,
        // so we downcast to recover which layer failed.
        let outcome = self.0.clone().oneshot(svc_req).await.map_err(|e| {
            if e.is::<tower::load_shed::error::Overloaded>() {
                // RESOURCE_EXHAUSTED is the standard gRPC backpressure code; load balancers
                // (e.g. Envoy, grpc-go) treat it as a retry signal and route elsewhere.
                Status::resource_exhausted("service overloaded")
            } else if e.is::<tower::timeout::error::Elapsed>() {
                Status::deadline_exceeded("request timeout")
            } else {
                Status::internal(e.to_string())
            }
        })?;

        // Infallible by construction; the match goes non-exhaustive (compile error)
        // if `ExecuteResponse`'s conversion ever becomes fallible.
        let Ok(response) = proto::ExecuteResponse::proto_try_from(outcome);

        Ok(Response::new(proto::Response {
            kind: Some(proto::response::Kind::Execute(response)),
        }))
    }
}

/// Backpressure and timeout policy for the gRPC service.
#[cfg_attr(
    feature = "serde",
    ::serde_with::serde_as,
    derive(::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Clone, Copy)]
pub struct GrpcConfig {
    /// Capacity of the bounded request queue. When full, requests are shed with
    /// `RESOURCE_EXHAUSTED`.
    pub connections_limit: usize,
    /// Maximum number of WASM executions running concurrently.
    pub max_parallel_wasm_executions: usize,
    /// Deadline for handling a single request end-to-end.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "request_handling_timeout_seconds"),
        serde_as(as = "::serde_with::DurationSeconds<u64>")
    )]
    pub request_handling_timeout: Duration,
}

const DEFAULT_MAX_PARALLEL_WASM_EXECUTIONS: usize = 2;
const DEFAULT_CONNECTIONS_LIMIT: usize = 500;
const DEFAULT_REQUEST_HANDLING_TIMEOUT: Duration = Duration::from_secs(30);

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            connections_limit: DEFAULT_CONNECTIONS_LIMIT,
            max_parallel_wasm_executions: DEFAULT_MAX_PARALLEL_WASM_EXECUTIONS,
            request_handling_timeout: DEFAULT_REQUEST_HANDLING_TIMEOUT,
        }
    }
}
