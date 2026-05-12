use defuse_crypto::Curve;
pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};
use digest::Digest;

use crate::Algorithm;

#[cfg(all(feature = "verify", feature = "near-contract"))]
type Sha256 = defuse_near_utils::digest::Sha256;

#[cfg(all(feature = "verify", not(feature = "near-contract"), feature="sha2"))]
type Sha256 = sha2::Sha256;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[derive(Debug, Clone)]
pub struct P256;

#[cfg(all(feature = "verify", any(feature = "near-contract", feature = "sha2")))]
impl Algorithm for P256 {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        // Use host impl of SHA-256 here to reduce gas consumption
        let prehashed = <Sha256 as Digest>::digest(msg).into();

        defuse_crypto::P256::verify(&signature.0, &prehashed, &public_key.0).is_some()
    }
}
