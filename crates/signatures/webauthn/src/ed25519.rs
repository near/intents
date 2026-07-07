use defuse_signature_schema::{Result, Schema};

use crate::Algorithm;

/// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// ed25519 curve
#[derive(Debug, Clone)]
pub struct Ed25519;

impl<M> Schema<M> for Ed25519
where
    M: AsRef<[u8]>,
{
    type Output = M;

    fn derive(&self, input: M) -> Result<Self::Output> {
        Ok(input)
    }
}

impl Algorithm for Ed25519 {
    type Curve = defuse_kdf_crypto::Ed25519;
}
