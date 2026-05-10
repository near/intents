use std::marker::PhantomData;

pub use defuse_crypto::{P256CompressedPublicKey, P256Signature};

use crate::AlgorithmPrehash;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct P256<D: digest::Digest>(PhantomData<D>);

impl<D: digest::Digest> AlgorithmPrehash for P256<D> {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;
    type Digest = D;

    #[inline]
    fn verify_prehash(
        prehash: &[u8],
        public_key: &Self::PublicKey,
        signature: &Self::Signature,
    ) -> bool {
        let Ok(prehash) = prehash.try_into() else {
            return false;
        };
        <defuse_crypto::P256 as defuse_crypto::Curve>::verify(&signature.0, prehash, &public_key.0)
            .is_some()
    }
}
