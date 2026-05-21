use bytes::Bytes;
use defuse_outlayer_executor::Outcome;

use crate::{Code, Error, Outlayer};

impl tower::Service<(Code<'static>, Bytes)> for Outlayer {
    type Response = Outcome;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Outcome, Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, (app, input): (Code<'static>, Bytes)) -> Self::Future {
        let outlayer = self.clone();
        Box::pin(async move { outlayer.execute(app, input, None).await })
    }
}
