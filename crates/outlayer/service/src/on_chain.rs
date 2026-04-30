use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use reqwest::Client;
use thiserror::Error;
use tower::Service;
use tracing::Instrument as _;
use url::Url;

use crate::types::{AccountId, ExecutionRequest, OffChainRequest};

#[derive(Debug, Error)]
pub enum OnChainFetchError {
    #[error("RPC request failed: {0}")]
    Rpc(#[from] reqwest::Error),
    #[error("project not found: {0}")]
    NotFound(AccountId),
}

#[derive(Clone)]
pub struct OnChainFetchService {
    rpc_url: Url,
    client: Client,
}

impl OnChainFetchService {
    pub fn new(rpc_url: Url) -> Self {
        Self {
            rpc_url,
            client: Client::new(),
        }
    }
}

impl Service<OffChainRequest> for OnChainFetchService {
    type Response = ExecutionRequest;
    type Error = OnChainFetchError;
    type Future = BoxFuture<'static, Result<ExecutionRequest, OnChainFetchError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(level = "debug", name = "on_chain.fetch", skip_all)]
    fn call(&mut self, req: OffChainRequest) -> Self::Future {
        let client = self.client.clone();
        let rpc_url = self.rpc_url.clone();
        Box::pin(async move {
            let (wasm_url, wasm_hash) =
                fetch_project_wasm(&client, &rpc_url, &req.project_id).await?;

            Ok(ExecutionRequest {
                request_id: req.request_id,
                project_id: req.project_id,
                wasm_url,
                wasm_hash,
                input: req.input,
            })
        }.instrument(tracing::Span::current()))
    }
}

#[allow(clippy::unused_async)]
async fn fetch_project_wasm(
    _client: &Client,
    _rpc_url: &Url,
    _project_id: &AccountId,
) -> Result<(String, [u8; 32]), OnChainFetchError> {
    // TODO: implement NEAR RPC view call to fetch wasm_url and wasm_hash
    todo!("NEAR RPC fetch not yet implemented")
}
