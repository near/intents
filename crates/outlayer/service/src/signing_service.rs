use std::task::{Context, Poll};

use defuse_outlayer_crypto::{DeriveSigner, ed25519::Ed25519, signer::InMemorySigner};
use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_with::{hex::Hex, serde_as};
use tower::Service;

use crate::{
    error::ExecutionStackError,
    types::{ExecutionRequest, ExecutionResponse},
};

#[serde_as]
#[derive(Debug, Serialize)]
pub struct SignedExecutionResponse<T = ExecutionResponse> {
    pub response: T,
    #[serde_as(as = "Hex")]
    pub signature: [u8; 64],
}

// TODO: signing may eventually be lifted out of the Tower stack entirely
#[derive(Clone)]
pub struct SigningService<S> {
    inner: S,
    signing_key: InMemorySigner,
}

impl<S> SigningService<S> {
    pub const fn new(inner: S, signing_key: InMemorySigner) -> Self {
        Self { inner, signing_key }
    }
}

fn sign_response<T: Serialize>(key: &InMemorySigner, response: T) -> SignedExecutionResponse<T> {
    let json = serde_json::to_vec(&response).expect("response serialization is infallible");
    let signature =
        <InMemorySigner as DeriveSigner<Ed25519>>::derive_sign(key, &[0u8; 32], &json).to_bytes();
    SignedExecutionResponse { response, signature }
}

impl<S, Req> Service<Req> for SigningService<S>
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
        let execution_req: ExecutionRequest = req.into();
        Box::pin(async move {
            let response = inner.call(execution_req).await?;
            Ok(sign_response(&key, response))
        })
    }
}
