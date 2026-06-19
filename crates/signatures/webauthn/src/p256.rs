use defuse_crypto::VerifiableCurve;
pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};
use defuse_digest::{Digest, sha2::Sha256};

use crate::Algorithm;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[derive(Debug, Clone)]
pub struct P256;

impl Algorithm for P256 {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        let prehashed = Sha256::digest(msg).into();

        defuse_crypto::P256::verify(&signature.0, &prehashed, &public_key.0).is_some()
    }
}
