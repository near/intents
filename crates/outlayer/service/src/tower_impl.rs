use std::task::{Context, Poll};

use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt};

use crate::{Code, Error, Outlayer};
use defuse_outlayer_executor::Outcome;

pub struct ExecuteRequest {
    pub app: Code<'static>,
    pub input: Bytes,
    pub fuel: Option<u64>,
}

impl tower::Service<ExecuteRequest> for Outlayer {
    type Response = Outcome;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Outcome, Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ExecuteRequest) -> Self::Future {
        let this = self.clone();
        async move { this.execute(req.app, req.input, req.fuel).await }.boxed()
    }
}
