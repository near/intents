use std::sync::Arc;

use defuse_outlayer_executor::Signer;
use defuse_outlayer_proto as proto;
use defuse_outlayer_proto::outlayer_service_server::{OutlayerService, OutlayerServiceServer};
use tonic::{Request, Response, Status};

use crate::{Code, Outlayer, OutlayerBuilder};

#[derive(Clone)]
struct OutlayerGrpc {
    outlayer: Outlayer,
}

#[tonic::async_trait]
impl OutlayerService for OutlayerGrpc {
    async fn execute(
        &self,
        request: Request<proto::ExecuteRequest>,
    ) -> Result<Response<proto::ExecuteResponse>, Status> {
        let req = request.into_inner();

        let code: Code<'static> = req
            .app
            .ok_or_else(|| Status::invalid_argument("missing app"))?
            .try_into()
            .map_err(|e: String| Status::invalid_argument(e))?;

        let outcome = self
            .outlayer
            .execute(code, req.input.into(), req.fuel)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(outcome.into()))
    }
}

impl OutlayerBuilder {
    pub fn build_service(
        self,
        signer: impl Into<Arc<dyn Signer>>,
    ) -> anyhow::Result<OutlayerServiceServer<impl OutlayerService + Clone>> {
        let outlayer = self.build(signer)?;
        Ok(OutlayerServiceServer::new(OutlayerGrpc { outlayer }))
    }
}
