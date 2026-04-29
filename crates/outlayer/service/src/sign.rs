use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;
use serde_with::{hex::Hex, serde_as};
use sha2::{Digest, Sha256};

use crate::types::ExecutionResponse;

/// Any response `T` paired with an ed25519 signature over the SHA-256 of its
/// JSON serialization.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct SignedExecutionResponse<T = ExecutionResponse> {
    pub response: T,
    #[serde_as(as = "Hex")]
    pub signature: [u8; 64],
}

/// Worker's ed25519 signing key, cheaply cloneable via `Arc`.
#[derive(Clone)]
pub struct WorkerSigningKey(pub Arc<SigningKey>);

impl WorkerSigningKey {
    pub fn sign_bytes(&self, data: &[u8]) -> [u8; 64] {
        let hash = Sha256::digest(data);
        self.0.sign(&hash).to_bytes()
    }

    pub fn sign<T: Serialize>(&self, response: T) -> SignedExecutionResponse<T> {
        let json = serde_json::to_vec(&response).expect("response serialization is infallible");
        let signature = self.sign_bytes(&json);
        SignedExecutionResponse {
            response,
            signature,
        }
    }
}
