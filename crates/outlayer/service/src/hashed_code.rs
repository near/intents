use bytes::Bytes;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct HashedCode {
    bytes: Bytes,
    hash: [u8; 32],
}

impl HashedCode {
    pub fn new(bytes: Bytes) -> Self {
        let hash = Sha256::digest(&bytes).into();
        Self { bytes, hash }
    }

    pub fn from_parts(bytes: Bytes, hash: [u8; 32]) -> Result<Self, HashMismatch> {
        let actual: [u8; 32] = Sha256::digest(&bytes).into();
        if actual != hash {
            return Err(HashMismatch);
        }
        Ok(Self { bytes, hash })
    }

    pub fn bytes(&self) -> Bytes {
        self.bytes.clone()
    }

    pub const fn hash(&self) -> [u8; 32] {
        self.hash
    }
}

#[derive(Debug, thiserror::Error)]
#[error("hash mismatch")]
pub struct HashMismatch;
