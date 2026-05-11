use std::marker::PhantomData;

pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};
use digest::Digest;

use crate::AlgorithmPrehash;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
pub struct CurveWithDigest<C: defuse_crypto::Curve, D: digest::Digest>(PhantomData<(D, C)>);
pub type P256<T> = CurveWithDigest<defuse_crypto::P256, T>;

impl<D> AlgorithmPrehash for CurveWithDigest<defuse_crypto::P256, D>
where
    D: Digest<OutputSize = digest::consts::U32>,
{
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;
    type Digest = D;

    #[inline]
    fn verify_prehash(
        prehash: [u8; 32],
        public_key: &Self::PublicKey,
        signature: &Self::Signature,
    ) -> bool {
        <defuse_crypto::P256 as defuse_crypto::Curve>::verify(&signature.0, &prehash, &public_key.0)
            .is_some()
    }
}
