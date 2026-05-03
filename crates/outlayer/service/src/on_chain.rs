use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use near_api::{NetworkConfig, RPCEndpoint};
use thiserror::Error;
use tower::Service;
use tracing::Instrument as _;
use url::Url;

use crate::outlayer_app_client::{OutlayerAppClient, OutlayerAppClientError};
use crate::types::{AccountId, ExecutionRequest, OffChainRequest};

#[derive(Debug, Error)]
pub enum OnChainFetchError {
    #[error("on-chain lookup failed: {0}")]
    Lookup(anyhow::Error),
    #[error("project not found: {0}")]
    NotFound(AccountId),
}

impl From<OutlayerAppClientError> for OnChainFetchError {
    fn from(e: OutlayerAppClientError) -> Self {
        Self::Lookup(anyhow::anyhow!(e))
    }
}

#[derive(Clone)]
pub struct OnChainFetchService {
    network_config: NetworkConfig,
}

impl OnChainFetchService {
    pub fn new(rpc_url: Url) -> Self {
        let network_config = NetworkConfig {
            rpc_endpoints: vec![RPCEndpoint::new(rpc_url)],
            ..NetworkConfig::mainnet()
        };
        Self { network_config }
    }

    pub const fn with_network_config(network_config: NetworkConfig) -> Self {
        Self { network_config }
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
        let network_config = self.network_config.clone();
        Box::pin(
            async move {
                let (wasm_url, wasm_hash) =
                    fetch_project_wasm(&network_config, &req.project_id).await?;

                Ok(ExecutionRequest {
                    request_id: req.request_id,
                    project_id: req.project_id,
                    wasm_url,
                    wasm_hash,
                    input: req.input,
                })
            }
            .instrument(tracing::Span::current()),
        )
    }
}

async fn fetch_project_wasm(
    network_config: &NetworkConfig,
    project_id: &AccountId,
) -> Result<(String, [u8; 32]), OnChainFetchError> {
    let account_id: near_sdk::AccountId = project_id
        .0
        .parse()
        .map_err(|_| OnChainFetchError::NotFound(project_id.clone()))?;
    let client = OutlayerAppClient::new(account_id, network_config.clone());
    let code_hash = client.oa_code_hash().await?;
    let code_url = client.oa_code_url().await?;
    Ok((code_url, code_hash))
}
