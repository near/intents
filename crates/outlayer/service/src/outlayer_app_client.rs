use near_api::{Contract, NetworkConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutlayerAppClientError {
    #[error("RPC view call failed: {0}")]
    ViewCall(anyhow::Error),
    #[error("invalid code hash: expected 32 bytes")]
    InvalidHash,
}

pub struct OutlayerAppClient {
    account_id: near_sdk::AccountId,
    network_config: NetworkConfig,
}

impl OutlayerAppClient {
    pub const fn new(account_id: near_sdk::AccountId, network_config: NetworkConfig) -> Self {
        Self {
            account_id,
            network_config,
        }
    }

    // TODO: use generated bindings from contract ABI instead of manual JSON deserialization
    pub async fn oa_code_hash(&self) -> Result<[u8; 32], OutlayerAppClientError> {
        let hash_hex: String = Contract(self.account_id.clone())
            .call_function("oa_code_hash", ())
            .read_only()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(|e| OutlayerAppClientError::ViewCall(e.into()))?;
        let bytes =
            hex::decode(&hash_hex).map_err(|e| OutlayerAppClientError::ViewCall(e.into()))?;
        bytes
            .try_into()
            .map_err(|_| OutlayerAppClientError::InvalidHash)
    }

    // TODO: use generated bindings from contract ABI instead of manual JSON deserialization
    pub async fn oa_code_url(&self) -> Result<String, OutlayerAppClientError> {
        Contract(self.account_id.clone())
            .call_function("oa_code_url", ())
            .read_only()
            .fetch_from(&self.network_config)
            .await
            .map(|d| d.data)
            .map_err(|e| OutlayerAppClientError::ViewCall(e.into()))
    }
}
