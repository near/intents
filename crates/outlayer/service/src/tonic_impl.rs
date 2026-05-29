use defuse_outlayer_executor::Outcome;
use defuse_outlayer_proto as proto;
use defuse_outlayer_proto::outlayer_service_server::OutlayerService;
use tonic::{Request, Response, Status};
use tower::ServiceExt as _;

use crate::tower_impl::ExecuteRequest;

#[derive(Clone)]
pub struct OutlayerGrpc<S> {
    service: S,
}

impl<S> OutlayerGrpc<S> {
    pub const fn new(service: S) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl<S> OutlayerService for OutlayerGrpc<S>
where
    S: tower::Service<ExecuteRequest, Response = Outcome> + Clone + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    S::Future: Send,
{
    async fn call(
        &self,
        request: Request<proto::Request>,
    ) -> Result<Response<proto::Response>, Status> {
        let svc_req: ExecuteRequest = request
            .into_inner()
            .try_into()
            .map_err(|e: anyhow::Error| Status::invalid_argument(e.to_string()))?;

        let outcome = self.service.clone().oneshot(svc_req).await.map_err(|e| {
            let e: Box<dyn std::error::Error + Send + Sync + 'static> = e.into();
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

        Ok(Response::new(proto::Response {
            kind: Some(proto::response::Kind::Execute(outcome.into())),
        }))
    }
}
