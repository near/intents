use std::sync::Arc;

use ed25519_dalek::{Signature, Signer, SigningKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::types::ExecutionResponse;

/// Any response `T` paired with an ed25519 signature over the SHA-256 of its
/// JSON serialization.
#[derive(Debug)]
pub struct SignedExecutionResponse<T = ExecutionResponse> {
    pub response: T,
    pub signature: Signature,
}

/// Worker's ed25519 signing key, cheaply cloneable via `Arc`.
#[derive(Clone)]
pub struct WorkerSigningKey(pub Arc<SigningKey>);

impl WorkerSigningKey {
    pub fn sign<T: Serialize>(&self, response: T) -> SignedExecutionResponse<T> {
        let json =
            serde_json::to_vec(&response).expect("response serialization is infallible");
        let hash = Sha256::digest(&json);
        let signature = self.0.sign(&hash);
        SignedExecutionResponse { response, signature }
    }
}
