use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;

use crate::{Code, Error, Outlayer};
use defuse_outlayer_executor::Outcome;

#[derive(Debug)]
pub struct ExecuteRequest {
    pub app: Code<'static>,
    pub input: Bytes,
    pub fuel: Option<u64>,
}

#[cfg(feature = "proto")]
impl TryFrom<defuse_outlayer_proto::ExecuteRequest> for ExecuteRequest {
    type Error = anyhow::Error;

    fn try_from(p: defuse_outlayer_proto::ExecuteRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            app: p
                .app
                .ok_or_else(|| anyhow::anyhow!("missing app"))?
                .try_into()?,
            input: p.input.into(),
            fuel: p.fuel,
        })
    }
}

#[cfg(feature = "proto")]
impl TryFrom<defuse_outlayer_proto::Request> for ExecuteRequest {
    type Error = anyhow::Error;

    fn try_from(p: defuse_outlayer_proto::Request) -> Result<Self, Self::Error> {
        match p
            .kind
            .ok_or_else(|| anyhow::anyhow!("missing request kind"))?
        {
            defuse_outlayer_proto::request::Kind::Execute(req) => req.try_into(),
        }
    }
}

impl tower::Service<ExecuteRequest> for Outlayer {
    type Response = Outcome;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Outcome, Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ExecuteRequest) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.execute(req.app, req.input, req.fuel).await })
    }
}
