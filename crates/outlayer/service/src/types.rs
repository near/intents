use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use ed25519_dalek::{Signature, SigningKey};
use serde::Deserialize;
use serde_with::{hex::Hex, serde_as};
use thiserror::Error;

/// Opaque NEAR account identifier. Wraps a validated string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
    /// Transaction hash that originated this request on-chain.
    #[serde_as(as = "Hex")]
    pub tx_hash: [u8; 32],
    pub project_id: AccountId,
    #[serde_as(as = "serde_with::base64::Base64")]
    pub input: Bytes,
    #[serde_as(as = "Hex")]
    pub wasm_hash: [u8; 32],
    /// URL to fetch the WASM binary from.
    pub wasm_url: String,
}

/// Worker-controlled resource caps. Never caller-controlled.
#[derive(Debug, Clone, Default)]
pub struct ResourceLimits;

/// Compiled WASM module ready for execution.
///
/// Placeholder — replace with `wasmtime::Module` (or equivalent) when the
/// runtime is integrated. Populated by `WasmCacheLayer` via an LRU cache
/// keyed by `wasm_hash`; `WasmExecutor` executes it directly.
#[derive(Debug, Clone, Default)]
pub struct CompiledWasmRuntime;

/// Placeholder for on-chain project storage (key → value snapshot).
#[derive(Debug, Clone, Default)]
pub struct ProjectStorage;

/// Placeholder for decrypted project environment variables / secrets.
#[derive(Debug, Clone, Default)]
pub struct ProjectEnv;

/// Fully normalised execution request consumed by the inner Tower stack.
/// All inputs are resolved; WASM is already compiled.
#[derive(Debug, Clone)]
pub struct WasmExecutionRequest {
    pub request: OnChainRequest,
    pub runtime: CompiledWasmRuntime,
    pub storage: ProjectStorage,
    pub env: ProjectEnv,
    pub caller: Option<AccountId>,
    pub limits: ResourceLimits,
}

/// Target NEAR contract for submitting the execution response.
#[derive(Debug, Clone)]
pub struct OnChainDestination {
    pub contract_id: AccountId,
}

#[derive(Debug, Default)]
pub struct ExecutionMetrics {
    pub instructions_used: u64,
    pub wall_time: Duration,
    pub compile_time: Option<Duration>,
}

/// WASM-level execution failure. Returned inside `ExecutionResponse::output`,
/// not as a tower `Service::Error` — `WasmExecutor` is `Infallible`.
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("resource limit exceeded")]
    ResourceLimit,
    #[error("wasm trap: {0}")]
    Trap(String),
    #[error("storage error: {0}")]
    Storage(String),
}

/// Raw output from `WasmExecutor`. No signature — `SigningLayer` wraps this
/// into `SignedExecutionResponse` before it leaves the inner stack.
pub struct ExecutionResponse {
    pub request: OnChainRequest,
    /// Raw stdout, capped at `limits.max_output_bytes`. Caller decodes.
    pub output: Result<Bytes, ExecutionError>,
    pub metrics: ExecutionMetrics,
    /// Storage state after execution, to be submitted on-chain by the caller.
    pub storage: ProjectStorage,
    pub respond_to: OnChainDestination,
}

/// `ExecutionResponse` after signing. The type guarantees the signature is
/// present — there is no intermediate state where `signature` is uninitialised.
pub struct SignedExecutionResponse {
    pub response: ExecutionResponse,
    /// Worker ed25519 signature over `request.tx_hash`. Proves this worker
    /// processed exactly that request. Valid for both success and error responses.
    pub signature: Signature,
}

/// Worker's ed25519 signing key, cheaply cloneable via `Arc`.
///
/// The worker's public key is registered on-chain. Callers verify
/// `SignedExecutionResponse::signature` against it to confirm which worker
/// handled their request.
///
/// TODO: replace with the signing key type from the existing codebase once integrated.
#[derive(Clone)]
pub struct WorkerSigningKey(pub Arc<SigningKey>);

impl WorkerSigningKey {
    pub fn sign(&self, response: ExecutionResponse) -> SignedExecutionResponse {
        use ed25519_dalek::Signer;
        let signature = self.0.sign(&response.request.tx_hash);
        SignedExecutionResponse { response, signature }
    }
}

// ── Network-facing request types ─────────────────────────────────────────────

/// Cryptographic proof that `caller` controls the stated NEAR account.
///
/// The signature covers `sha256(request_id.as_bytes())`, binding the proof to
/// this specific request. `ProjectContextLayer` verifies:
///   1. the signature is valid for `public_key`
///   2. `public_key` is a full-access key for `caller` (NEAR RPC check)
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct IdentityProof {
    /// ed25519 public key (compressed, 32 bytes). Hex-encoded in JSON.
    #[serde_as(as = "Hex")]
    pub public_key: [u8; 32],
    /// ed25519 signature over `sha256(request_id.as_bytes())`. Hex-encoded in JSON.
    #[serde_as(as = "Hex")]
    pub signature: [u8; 64],
}

/// Network-facing execution request.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    OnChain(OnChainRequest),
    /// Placeholder — offchain request format TBD.
    OffChain,
}
