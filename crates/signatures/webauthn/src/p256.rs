use defuse_digest::{Digest, sha2::Sha256};

use crate::Algorithm;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
pub struct P256;

impl Algorithm for P256 {
    type Curve = defuse_kdf_crypto::p256::P256;

    fn derive(msg: impl AsRef<[u8]>) -> impl AsRef<[u8]> {
        // prehash via SHA-256
        Sha256::digest(msg)
    }
}

// TODO: tests