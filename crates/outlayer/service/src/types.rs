use std::fmt::Write as _;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

/// Opaque NEAR account identifier. Wraps a validated string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// All data carried by an on-chain execution request.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct OnChainRequest {
    /// NOTE: its sitll not clear how to uniquely identify a request on chain. Lets
    /// use placeholder for now ...
    #[serde_as(as = "Hex")]
    pub nonce: [u8; 32],
    #[serde_as(as = "serde_with::base64::Base64")]
    pub tx_hash: [u8; 32],
    pub project_id: AccountId,
    #[serde_as(as = "serde_with::base64::Base64")]
    pub input: Bytes,
    #[serde_as(as = "Hex")]
    pub wasm_hash: [u8; 32],
    pub wasm_url: String,
}

/// Simplified, internal execution request passed to `OutlayerService`.
/// Contains only what WASM execution needs.
#[derive(Debug, Clone)]
pub struct OffChainRequest {
    /// Opaque identifier for this request, used for logging only.
    pub request_id: String,
    pub project_id: AccountId,
    pub input: Bytes,
}

/// Simplified, internal execution request passed to `OutlayerService`.
/// Contains only what WASM execution needs.
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    /// Opaque identifier for this request, used for logging only.
    pub request_id: String,
    pub project_id: AccountId,
    pub wasm_url: String,
    pub wasm_hash: [u8; 32],
    pub input: Bytes,
}

impl From<OnChainRequest> for ExecutionRequest {
    fn from(r: OnChainRequest) -> Self {
        Self {
            request_id: r.nonce.iter().fold(String::with_capacity(64), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            }),
            project_id: r.project_id,
            wasm_url: r.wasm_url,
            wasm_hash: r.wasm_hash,
            input: r.input,
        }
    }
}

/// Raw output from `WasmExecutor`. Contains only execution results.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    // TODO: error string may be arbitrarily large; consider a dedicated SerializeAs that trims it
    #[serde_as(as = "Result<serde_with::base64::Base64, serde_with::DisplayFromStr>")]
    pub result: anyhow::Result<Bytes>,
    #[serde_as(as = "serde_with::base64::Base64")]
    pub logs: Bytes,

    pub metrics: ExecutionMetrics,
    pub storage: ProjectStorage,
}

/// Placeholder for on-chain project storage (key → value snapshot).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectStorage;

/// Placeholder for decrypted project environment variables / secrets.
#[derive(Debug, Clone, Default)]
pub struct ProjectEnv;

#[derive(Debug, Default, Serialize)]
pub struct ExecutionMetrics {
    pub instructions_used: u64,
}
