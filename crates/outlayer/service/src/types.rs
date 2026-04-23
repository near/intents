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

/// Network-facing execution request.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    OnChain(OnChainRequest),
    /// Placeholder — offchain request format TBD.
    OffChain,
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

/// Raw output from `WasmExecutor`. Signed via `map_response(sign)` before
/// leaving the stack.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub project_id: AccountId,
    #[serde_as(as = "Hex")]
    pub wasm_hash: [u8; 32],
    #[serde_as(as = "Hex")]
    pub nonce: [u8; 32],

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
