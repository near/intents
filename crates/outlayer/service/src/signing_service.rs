use std::marker::PhantomData;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use tower::Service;

use crate::{
    error::ExecutionStackError,
    sign::{SignedExecutionResponse, WorkerSigningKey},
    types::{ExecutionRequest, ExecutionResponse},
};

/// Tower service that wraps `OutlayerService`, accepting any request type
/// convertible into `ExecutionRequest`. Holds the original request alongside
/// the `ExecutionResponse` at call time, giving full flexibility over what
/// gets included in the signature.
pub struct SigningService<S, Req> {
    inner: S,
    signing_key: WorkerSigningKey,
    _req: PhantomData<fn(Req)>,
}

impl<S, Req> SigningService<S, Req> {
    pub fn new(inner: S, signing_key: WorkerSigningKey) -> Self {
        Self {
            inner,
            signing_key,
            _req: PhantomData,
        }
    }
}

impl<S, Req> Clone for SigningService<S, Req>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            signing_key: self.signing_key.clone(),
            _req: PhantomData,
        }
    }
}

impl<S, Req> Service<Req> for SigningService<S, Req>
where
    Req: Into<ExecutionRequest> + Clone + Send + 'static,
    S: Service<ExecutionRequest, Response = ExecutionResponse, Error = ExecutionStackError>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = SignedExecutionResponse;
    type Error = ExecutionStackError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let mut inner = self.inner.clone();
        let key = self.signing_key.clone();
        // `req` is available here (and its Clone) to incorporate any of its fields
        // in the signing payload if needed — see `WorkerSigningKey::sign_bytes`.
        let execution_req: ExecutionRequest = req.into();
        Box::pin(async move {
            let response = inner.call(execution_req).await?;
            Ok(key.sign(response))
        })
    }
}
