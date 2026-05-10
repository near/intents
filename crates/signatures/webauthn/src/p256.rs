pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};

use crate::Algorithm;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct P256;

impl Algorithm for P256 {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        use sha2::Digest as _;
        // Use host impl of SHA-256 here to reduce gas consumption
        #[cfg(feature = "near-contract")]
        let prehashed = defuse_near_utils::digest::Sha256::digest(msg).into();
        #[cfg(not(feature = "near-contract"))]
        let prehashed = sha2::Sha256::digest(msg).into();

        <defuse_crypto::P256 as defuse_crypto::Curve>::verify(
            &signature.0,
            &prehashed,
            &public_key.0,
        )
        .is_some()
    }
}
