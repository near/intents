use std::marker::PhantomData;

pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};
use digest::Digest;

use crate::{AlgorithmPrehash, HasSignature};

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[derive(Debug, Clone)]
pub struct P256;

impl HasSignature for P256 {
    type Signature = P256Signature;
}

impl AlgorithmPrehash for P256 {
    type PublicKey = P256CompressedPublicKey;

    #[inline]
    fn verify_digest(
        prehash: [u8; 32],
        public_key: &Self::PublicKey,
        signature: &Self::Signature,
    ) -> bool {
        <defuse_crypto::P256 as defuse_crypto::Curve>::verify(&signature.0, &prehash, &public_key.0)
            .is_some()
    }
}
